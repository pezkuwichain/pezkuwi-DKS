// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Random sequences of citizenship calls against `identity-kyc` and `referral` together.
//!
//! The two pallets keep the citizen register between them: one holds who is a citizen and under
//! which identity hash, the other who vouched for whom and how often. Each keeps counters that
//! must agree with its own records, and each says so in `try_state`. A counter kept by hand
//! fails silently, on whichever path forgot to update it, so the way to find that path is to walk
//! many of them. After every step this checks both pallets' `try_state` and that the total
//! issuance has not moved: these pallets reserve and unreserve deposits and must never mint or
//! burn.
//!
//! Each input is one fresh chain and one sequence of up to 64 steps drawn from it, so a crashing
//! input reproduces on its own.
//!
//! # Running
//!
//! `cargo hfuzz run citizenship` from this directory; `cargo hfuzz run-debug citizenship
//! hfuzz_workspace/citizenship/*.fuzz` to replay a crash.

#[cfg(all(fuzzing, not(feature = "try-runtime")))]
compile_error!("build with `--features try-runtime`: the pallets' `try_state` is what this checks");

use honggfuzz::fuzz;
use pezframe_support::{
	traits::{Hooks, UnfilteredDispatchable},
	weights::Weight,
};
use pezpallet_identity_kyc::Call as KycCall;
use pezpallet_referral::{
	mock::{new_test_ext, AccountId, Balances, Referral, RuntimeOrigin, System, Test},
	Call as ReferralCall,
};
use pezsp_core::H256;
use rand::{rngs::SmallRng, Rng, SeedableRng};

/// Few enough accounts and identity hashes that calls keep landing on the same ones: the
/// interesting states are an account that applied twice, or two accounts with one hash.
const ACCOUNTS: AccountId = 12;
const HASHES: u64 = 8;
const MAX_STEPS: usize = 64;

fn account(rng: &mut SmallRng) -> AccountId {
	// The mock's founder is account 100; it has to be reachable as referrer and as target.
	if rng.gen_ratio(1, 8) {
		pezpallet_referral::mock::FOUNDER
	} else {
		rng.gen_range(1..=ACCOUNTS)
	}
}

fn maybe_account(rng: &mut SmallRng) -> Option<AccountId> {
	rng.gen::<bool>().then(|| account(rng))
}

/// Mostly a signed origin, sometimes root (the governance origin in the mock), rarely none.
fn origin(rng: &mut SmallRng) -> RuntimeOrigin {
	match rng.gen_range(0..16) {
		0 => RuntimeOrigin::none(),
		1..=3 => RuntimeOrigin::root(),
		_ => RuntimeOrigin::signed(account(rng)),
	}
}

fn step(rng: &mut SmallRng) {
	let origin = origin(rng);
	// The result is not the point: most random calls are refused, and a refusal is correct.
	// What must hold is the state afterwards, which the caller checks.
	let _ = match rng.gen_range(0..9) {
		0 => KycCall::<Test>::apply_for_citizenship {
			identity_hash: H256::from_low_u64_be(rng.gen_range(0..HASHES)),
			referrer: maybe_account(rng),
			inviter: maybe_account(rng),
		}
		.dispatch_bypass_filter(origin),
		1 => KycCall::<Test>::approve_referral { applicant: account(rng) }
			.dispatch_bypass_filter(origin),
		2 => KycCall::<Test>::confirm_citizenship {}.dispatch_bypass_filter(origin),
		3 => {
			KycCall::<Test>::revoke_citizenship { who: account(rng) }.dispatch_bypass_filter(origin)
		},
		4 => KycCall::<Test>::restore_citizenship { who: account(rng) }
			.dispatch_bypass_filter(origin),
		5 => KycCall::<Test>::renounce_citizenship {}.dispatch_bypass_filter(origin),
		6 => KycCall::<Test>::cancel_application {}.dispatch_bypass_filter(origin),
		7 => ReferralCall::<Test>::initiate_referral { referred: account(rng) }
			.dispatch_bypass_filter(origin),
		_ => ReferralCall::<Test>::withdraw_referral { referred: account(rng) }
			.dispatch_bypass_filter(origin),
	};

	// Time passes now and then: the vouching wait is 10 blocks and the referral fallback 100 in
	// the mock, so a jump of up to 60 crosses both within a sequence.
	if rng.gen_ratio(1, 6) {
		let now = System::block_number() + rng.gen_range(1..=60);
		System::set_block_number(now);
		// The cleanup queue is drained in `on_idle`, a little or a lot at a time.
		let budget = Weight::from_parts(rng.gen_range(0..2_000_000_000), u64::MAX);
		<Referral as Hooks<u64>>::on_idle(now, budget);
	}
	System::reset_events();
}

fn main() {
	loop {
		fuzz!(|seed: [u8; 32]| {
			let mut rng = SmallRng::from_seed(seed);
			new_test_ext().execute_with(|| {
				for who in 1..=ACCOUNTS {
					let _ = Balances::force_set_balance(RuntimeOrigin::root(), who, 10_000);
				}
				let issuance = pezpallet_balances::TotalIssuance::<Test>::get();

				for _ in 0..rng.gen_range(1..=MAX_STEPS) {
					step(&mut rng);

					#[cfg(feature = "try-runtime")]
					{
						let now = System::block_number();
						<pezpallet_referral::mock::IdentityKyc as Hooks<u64>>::try_state(now)
							.unwrap();
						<Referral as Hooks<u64>>::try_state(now).unwrap();
					}
					assert_eq!(
						pezpallet_balances::TotalIssuance::<Test>::get(),
						issuance,
						"the citizenship pallets minted or burnt"
					);
				}
			});
		});
	}
}
