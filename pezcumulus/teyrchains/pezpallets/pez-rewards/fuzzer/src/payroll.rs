// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Random histories of the citizens' monthly payroll.
//!
//! Each month the pallet fixes a rate against the trust roll and freezes the roll, opens a claim
//! window, and pays each claim by instructing the Asset Hub over XCM. It can only instruct, never
//! mint, so what matters is that it never instructs more than it has: `try_state` holds the
//! running totals, and this fuzzer adds two checks of its own -- what a month's claims add up to
//! is at most what was available when that month was finalised, and one citizenship is paid at
//! most once a month, whichever of its accounts claims. The second is the one a reissue in the
//! middle of a window used to break.
//!
//! Each input is one fresh chain and one history of up to 96 steps: the system started from the
//! right and the wrong origin, funding reports that grow, stall and go backwards, claims by
//! anyone for any epoch, the trust roll and the parliamentary seats changing as they can in
//! production, citizenships reissued to new accounts in the middle of a window, XCM delivery
//! failing and recovering, and time jumping across epoch ends and claim deadlines -- sometimes
//! both in one block. Both checks run after every call and every block.
//!
//! The mock's trust roll is a table the fuzzer writes, so the fuzzer keeps to what the real roll
//! allows: while it is frozen a score can only drop to zero (a revoked citizenship), and a seat
//! is always recorded as taken now, never backdated.
//!
//! # Running
//!
//! `cargo hfuzz run payroll` from this directory with `HFUZZ_BUILD_ARGS="--features
//! try-runtime"`, which fuzz.yml sets.

#[cfg(all(fuzzing, not(feature = "try-runtime")))]
compile_error!("build with `--features try-runtime`: the pallet's `try_state` is what this checks");

use std::collections::BTreeMap;

use honggfuzz::fuzz;
use pezframe_support::traits::UnfilteredDispatchable;
use pezpallet_pez_rewards::{
	mock::{
		check_invariants, clear_seat, clear_sent_xcm, fail_sending, freezes, new_test_ext,
		run_to_block, set_seat, set_trust, AccountId, MockParliamentRoll, MockTrustRoll,
		PezRewards, RuntimeEvent, RuntimeOrigin, System, Test,
	},
	Call, ClaimedRewards, EpochInClaim, EpochInfo, Event, ParliamentRoll, ReportedIncentiveTotal,
	TrustRoll, BLOCKS_PER_EPOCH, CLAIM_PERIOD_BLOCKS,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};

const FIRST_ACCOUNTS: AccountId = 8;
const MAX_STEPS: usize = 96;

struct History {
	rng: SmallRng,
	/// Accounts that currently hold a citizenship; a reissue swaps one for a new one.
	accounts: Vec<AccountId>,
	next_account: AccountId,
	/// What each epoch had available when it was finalised, from its `EpochFinalized` event.
	available: BTreeMap<u32, u128>,
	/// Which citizenship each account holds or held: a reissue hands it to the new account.
	citizenship: BTreeMap<AccountId, AccountId>,
}

impl History {
	fn account(&mut self) -> AccountId {
		self.accounts[self.rng.gen_range(0..self.accounts.len())]
	}

	/// Root is the funding and force origin in the mock; a signed origin must be refused.
	fn origin(&mut self) -> RuntimeOrigin {
		if self.rng.gen_ratio(1, 5) {
			RuntimeOrigin::signed(self.account())
		} else {
			RuntimeOrigin::root()
		}
	}

	/// A cumulative funding report: mostly growing, sometimes repeated, sometimes backwards
	/// (which must be refused), sometimes enormous.
	fn funding_report(&mut self) -> u128 {
		let reported = ReportedIncentiveTotal::<Test>::get();
		match self.rng.gen_range(0..6) {
			0 => reported,
			1 => reported.saturating_sub(self.rng.gen_range(1..=1_000_000)),
			2 => u128::MAX - self.rng.gen_range(0..2),
			_ => reported.saturating_add(self.rng.gen_range(0..1_000_000_000_000_000_000)),
		}
	}

	/// Mostly the epoch whose window is open, since a claim against any other is refused
	/// before it reaches the payment; sometimes one either side of it, or any.
	fn claim_epoch(&mut self) -> u32 {
		let current = EpochInfo::<Test>::get().current_epoch;
		match (EpochInClaim::<Test>::get(), self.rng.gen_range(0..8)) {
			(Some(open), 0..=4) => open,
			(Some(open), 5) => open.saturating_add(1),
			(Some(open), 6) => open.saturating_sub(1),
			_ => self.rng.gen_range(0..=current.saturating_add(1)),
		}
	}

	/// Mostly an account the pallet would pay, so claims reach the payment; sometimes anyone.
	fn claimant(&mut self) -> AccountId {
		let owed: Vec<_> = self
			.accounts
			.iter()
			.copied()
			.filter(|who| {
				<MockTrustRoll as TrustRoll<AccountId, u64>>::score_of(who) > 0
					|| <MockParliamentRoll as ParliamentRoll<AccountId, u64>>::holds_seat(who)
			})
			.collect();
		if owed.is_empty() || self.rng.gen_ratio(1, 4) {
			self.account()
		} else {
			owed[self.rng.gen_range(0..owed.len())]
		}
	}

	fn roll_is_frozen() -> bool {
		freezes().into_iter().max().is_some_and(|until| System::block_number() <= until)
	}

	/// What `reissue_citizenship` does to the rolls this pallet reads, then what it does here.
	fn reissue(&mut self) {
		let index = self.rng.gen_range(0..self.accounts.len());
		let from = self.accounts[index];
		let to = self.next_account;
		self.next_account += 1;
		self.accounts[index] = to;
		let held = self.citizenship[&from];
		self.citizenship.insert(to, held);

		let score = <MockTrustRoll as TrustRoll<AccountId, u64>>::score_of(&from);
		set_trust(from, 0);
		set_trust(to, score);
		if let Some(at) = <MockParliamentRoll as ParliamentRoll<AccountId, u64>>::seated_at(&from) {
			let holds = <MockParliamentRoll as ParliamentRoll<AccountId, u64>>::holds_seat(&from);
			clear_seat(from);
			set_seat(to, at, holds);
		}
		PezRewards::rebind_account(&from, &to).unwrap();
	}

	fn step(&mut self) {
		// Refusals are expected and correct; what has to hold is the state afterwards.
		let _ = match self.rng.gen_range(0..10) {
			0 => Call::<Test>::initialize_rewards_system {}.dispatch_bypass_filter(self.origin()),
			1 => Call::<Test>::note_incentive_funding { total: self.funding_report() }
				.dispatch_bypass_filter(self.origin()),
			2 | 3 => {
				let epoch_index = self.claim_epoch();
				let who = self.claimant();
				Call::<Test>::claim_reward { epoch_index }
					.dispatch_bypass_filter(RuntimeOrigin::signed(who))
			},
			4 => {
				// A frozen roll only loses scores (a revocation); otherwise anything goes.
				let score = if Self::roll_is_frozen() || self.rng.gen_ratio(1, 4) {
					0
				} else {
					self.rng.gen_range(1..=1_000_000)
				};
				let who = self.account();
				set_trust(who, score);
				Ok(().into())
			},
			5 => {
				let (who, holds) = (self.account(), self.rng.gen());
				set_seat(who, System::block_number(), holds);
				Ok(().into())
			},
			6 => {
				let who = self.account();
				clear_seat(who);
				Ok(().into())
			},
			7 => {
				self.reissue();
				Ok(().into())
			},
			_ => {
				fail_sending(self.rng.gen_ratio(1, 3));
				Ok(().into())
			},
		};
		self.check();

		// Time moves on one step in three, so a claim window usually sees several steps.
		if self.rng.gen_ratio(1, 3) {
			// Aim at the edges: just before or at an epoch end or claim deadline; or a little,
			// staying inside a window; or anywhere.
			let epoch = BLOCKS_PER_EPOCH as u64;
			let window = CLAIM_PERIOD_BLOCKS as u64;
			let jump = match self.rng.gen_range(0..6) {
				0 => epoch - self.rng.gen_range(0..3),
				1 => window + self.rng.gen_range(0..3) - 1,
				2 => epoch + window,
				3 | 4 => self.rng.gen_range(1..=window / 8),
				_ => self.rng.gen_range(1..=2 * epoch),
			};
			let target = System::block_number() + jump;
			// Land one block short and run the last one, so the hooks see the boundary block.
			System::set_block_number(target - 1);
			run_to_block(target + self.rng.gen_range(0..3));
			self.check();
		}
		clear_sent_xcm();
		System::reset_events();
	}

	fn check(&mut self) {
		check_invariants();

		for record in System::events() {
			if let RuntimeEvent::PezRewards(Event::EpochFinalized {
				epoch_index, available, ..
			}) = record.event
			{
				self.available.insert(epoch_index, available);
			}
		}
		let mut claimed: BTreeMap<u32, u128> = BTreeMap::new();
		let mut drawn: BTreeMap<(u32, AccountId), AccountId> = BTreeMap::new();
		for (epoch, who, amount) in ClaimedRewards::<Test>::iter() {
			*claimed.entry(epoch).or_default() += amount;
			// One citizenship, one paid claim a month, whichever of its accounts made it.
			if amount > 0 {
				let held = self.citizenship[&who];
				if let Some(other) = drawn.insert((epoch, held), who) {
					panic!(
						"citizenship {held} was paid twice for epoch {epoch}, as {other} and {who}"
					);
				}
			}
		}
		for (epoch, total) in claimed {
			let available = self.available.get(&epoch).copied().unwrap_or(0);
			assert!(
				total <= available,
				"epoch {epoch} paid {total}, but only {available} was available when it closed"
			);
		}
	}
}

fn main() {
	loop {
		fuzz!(|seed: [u8; 32]| {
			let mut history = History {
				rng: SmallRng::from_seed(seed),
				accounts: (1..=FIRST_ACCOUNTS).collect(),
				next_account: 1_000,
				available: BTreeMap::new(),
				citizenship: (1..=FIRST_ACCOUNTS).map(|a| (a, a)).collect(),
			};
			new_test_ext().execute_with(|| {
				for _ in 0..history.rng.gen_range(1..=MAX_STEPS) {
					history.step();
				}
			});
		});
	}
}
