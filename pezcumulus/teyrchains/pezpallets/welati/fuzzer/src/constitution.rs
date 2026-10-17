// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Every welati call, in any order, from any origin.
//!
//! Welati is the constitution: elections and their calendar, the offices and their mandates, the
//! court's two halves, the house, the budget, citizens' initiatives. Its `try_state` holds the
//! rules that no single call is meant to break -- one election per office and only an unfinished
//! one, nobody seated in an elected office without a mandate, eleven judges split six and five
//! and the appointed half qualified -- and the pallets it drives (tiki, trust, identity-kyc,
//! referral) hold their own. A rule like that breaks on a sequence nobody wrote down, so this
//! walks sequences.
//!
//! The calls are not generated one by one: the input is decoded as SCALE-encoded welati calls,
//! so every call the pallet has is reachable, including any added later, and the fuzzer's
//! coverage and comparison feedback finds the call indices and the small account numbers the
//! mock's citizens use. Each call is preceded by two bytes, one choosing the origin and one how
//! many blocks pass first, during which the pallet's hooks run.
//!
//! Each input is one fresh chain -- the mock's citizens, a President and a seated house -- and
//! one sequence of up to 32 calls. After every block and every call, every pallet's `try_state`.
//!
//! # Running
//!
//! `cargo hfuzz run constitution` from this directory with `HFUZZ_BUILD_ARGS="--features
//! try-runtime"`, which fuzz.yml sets.

#[cfg(all(fuzzing, not(feature = "try-runtime")))]
compile_error!("build with `--features try-runtime`: the pallets' `try_state` is what this checks");

use codec::Decode;
use honggfuzz::fuzz;
use pezframe_support::traits::UnfilteredDispatchable;
use pezpallet_welati::{
	mock::{
		add_parliament_member, run_to_block, seat_president, ExtBuilder, RuntimeOrigin, System,
		Test,
	},
	Call,
};

const MAX_CALLS: usize = 32;
/// The mock funds accounts 1 to 110 and makes 1 to 12 citizens.
const ACCOUNTS: u8 = 20;

/// Root stands for every privileged origin the mock configures; a signed origin is the citizen
/// (or non-citizen) the call claims to come from.
fn origin(byte: u8) -> RuntimeOrigin {
	match byte % (ACCOUNTS + 2) {
		0 => RuntimeOrigin::root(),
		1 => RuntimeOrigin::none(),
		n => RuntimeOrigin::signed((n - 1) as u64),
	}
}

fn check() {
	#[cfg(feature = "try-runtime")]
	{
		use pezframe_support::traits::{TryState, TryStateSelect};
		<pezpallet_welati::mock::AllPalletsWithSystem as TryState<u64>>::try_state(
			System::block_number(),
			TryStateSelect::All,
		)
		.expect("try_state failed");
	}
}

fn main() {
	loop {
		fuzz!(|data: &[u8]| {
			ExtBuilder::default().build().execute_with(|| {
				seat_president(1);
				for member in 2..=12 {
					add_parliament_member(member);
				}
				check();

				let mut input = data;
				for _ in 0..MAX_CALLS {
					let [who, wait, rest @ ..] = input else { break };
					input = rest;
					let Ok(call) = Call::<Test>::decode(&mut input) else { break };

					// Most calls land in the same block; a high byte lets an election period
					// or a term run out first.
					let blocks = if *wait < 192 { 0 } else { u64::from(*wait - 191) * 20 };
					run_to_block(System::block_number() + blocks);

					// Refusals are expected and correct; what has to hold is the state after.
					let _ = call.dispatch_bypass_filter(origin(*who));
					check();
				}
			});
		});
	}
}
