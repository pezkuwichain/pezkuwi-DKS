// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Random histories of the PEZ release schedule.
//!
//! The treasury releases a fixed amount each month, halving every forty-eight months, and splits
//! each release between the incentive pot and the government pot. Nothing about that schedule is
//! accumulated: every figure the pallet stores has exactly one correct value that `try_state`
//! recomputes from the release index. So a history that ends in a wrong figure is a release that
//! paid what no month was owed, a month paid twice, or a period that advanced on its own -- and
//! with a fixed five billion, none of those can be undone afterwards.
//!
//! Each input is one fresh chain with the treasury allocation minted and one history of up to
//! 96 steps: activation from the right and the wrong origin, spends from both pots of any size,
//! XCM delivery failing and recovering, and time jumping by up to eight months and running a few
//! blocks, so a history reaches and crosses the first halving. The pallet's `try_state` runs after
//! every call and every block, and the PEZ supply must never move: the pallet only transfers.
//!
//! # Running
//!
//! `cargo hfuzz run schedule` from this directory with `HFUZZ_BUILD_ARGS="--features
//! try-runtime"`, which fuzz.yml sets.

#[cfg(all(fuzzing, not(feature = "try-runtime")))]
compile_error!("build with `--features try-runtime`: the pallet's `try_state` is what this checks");

use honggfuzz::fuzz;
use pezframe_support::traits::UnfilteredDispatchable;
use pezpallet_pez_treasury::{
	mock::{
		alice, bob, charlie, check_invariants, clear_sent_xcm, fail_sending, fund_treasury,
		jump_to_block, new_test_ext, pez_total_supply, run_blocks, RuntimeOrigin, System, Test,
	},
	Call, BLOCKS_PER_MONTH, TREASURY_ALLOCATION,
};
use pezsp_core::H256;
use rand::{rngs::SmallRng, Rng, SeedableRng};

const MAX_STEPS: usize = 96;
const MAX_MONTHS_PER_JUMP: u64 = 8;

fn account(rng: &mut SmallRng) -> H256 {
	match rng.gen_range(0..3) {
		0 => alice(),
		1 => bob(),
		_ => charlie(),
	}
}

/// Root is every privileged origin in the mock; a signed origin must always be refused.
fn origin(rng: &mut SmallRng) -> RuntimeOrigin {
	if rng.gen_ratio(1, 5) {
		RuntimeOrigin::signed(account(rng))
	} else {
		RuntimeOrigin::root()
	}
}

/// Amounts across the whole range that matters: dust, ordinary, a whole month's release, more
/// than any pot holds, and the edge of the type.
fn amount(rng: &mut SmallRng) -> u128 {
	match rng.gen_range(0..5) {
		0 => rng.gen_range(0..1_000),
		1 => rng.gen_range(0..1_000_000_000_000_000),
		2 => rng.gen_range(0..TREASURY_ALLOCATION / 48),
		3 => rng.gen_range(0..=TREASURY_ALLOCATION),
		_ => u128::MAX - rng.gen_range(0..2),
	}
}

fn step(rng: &mut SmallRng) {
	// Refusals are expected and correct; what has to hold is the state afterwards.
	let _ = match rng.gen_range(0..6) {
		0 => Call::<Test>::activate_distribution {}.dispatch_bypass_filter(origin(rng)),
		1 | 2 => Call::<Test>::spend_from_government_pot {
			beneficiary: account(rng),
			amount: amount(rng),
		}
		.dispatch_bypass_filter(origin(rng)),
		3 | 4 => {
			Call::<Test>::pay_from_incentive_pot { beneficiary: account(rng), amount: amount(rng) }
				.dispatch_bypass_filter(origin(rng))
		},
		_ => {
			// The release reports each month's incentive share to the rewards chain; a failed
			// send must not lose or double the release.
			fail_sending(rng.gen_ratio(1, 3));
			Ok(().into())
		},
	};
	check_invariants();

	if rng.gen_ratio(1, 2) {
		// Land on month boundaries as well as between them.
		let months = rng.gen_range(0..=MAX_MONTHS_PER_JUMP);
		let jitter = if rng.gen() { 0 } else { rng.gen_range(0..BLOCKS_PER_MONTH as u64) };
		jump_to_block(System::block_number() + months * BLOCKS_PER_MONTH as u64 + jitter);
		// One release is due per block once a month has passed, so a few blocks catch up a jump.
		run_blocks(rng.gen_range(1..=MAX_MONTHS_PER_JUMP + 4));
	}
	clear_sent_xcm();
	System::reset_events();
}

fn main() {
	loop {
		fuzz!(|seed: [u8; 32]| {
			let mut rng = SmallRng::from_seed(seed);
			fail_sending(false);
			new_test_ext().execute_with(|| {
				fund_treasury();
				let supply = pez_total_supply();
				for _ in 0..rng.gen_range(1..=MAX_STEPS) {
					step(&mut rng);
					assert_eq!(pez_total_supply(), supply, "the treasury minted or burnt PEZ");
				}
			});
		});
	}
}
