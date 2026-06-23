// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

//! Common runtime code for the Relay Chain, e.g. Pezkuwichain, Zagros, Pezkuwi, Dicle ...

#![cfg_attr(not(feature = "std"), no_std)]

pub mod assigned_slots;
pub mod auctions;
pub mod claims;
pub mod crowdloan;
pub mod elections;
pub mod identity_migrator;
pub mod impls;
pub mod paras_registrar;
pub mod paras_sudo_wrapper;
pub mod slot_range;
pub mod slots;
pub mod traits;

#[cfg(feature = "try-runtime")]
pub mod try_runtime;
pub mod xcm_sender;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod mock;

extern crate alloc;

use pezframe_support::{
	parameter_types,
	traits::{ConstU32, Currency, OneSessionHandler},
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use pezframe_system::limits;
use pezkuwi_primitives::{AssignmentId, Balance, BlockNumber, ValidatorId};
use pezsp_runtime::{FixedPointNumber, Perbill, Perquintill};
use static_assertions::const_assert;

pub use pezpallet_balances::Call as BalancesCall;
#[cfg(feature = "std")]
pub use pezpallet_staking::StakerStatus;
pub use pezpallet_timestamp::Call as TimestampCall;
use pezpallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
pub use pezsp_runtime::traits::Bounded;
#[cfg(any(feature = "std", test))]
pub use pezsp_runtime::BuildStorage;

/// Implementations of some helper traits passed into runtime modules as associated types.
pub use impls::ToAuthor;

#[deprecated(
	note = "Please use fungible::Credit instead. This type will be removed some time after March 2024."
)]
pub type NegativeImbalance<T> = <pezpallet_balances::Pezpallet<T> as Currency<
	<T as pezframe_system::Config>::AccountId,
>>::NegativeImbalance;

/// We assume that an on-initialize consumes 1% of the weight on average, hence a single extrinsic
/// will not be allowed to consume more than `AvailableBlockRatio - 1%`.
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(1);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
/// The storage proof size is not limited so far.
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

const_assert!(NORMAL_DISPATCH_RATIO.deconstruct() >= AVERAGE_ON_INITIALIZE_RATIO.deconstruct());

// Common constants used in all runtimes.
parameter_types! {
	pub const BlockHashCount: BlockNumber = 4096;
	/// The portion of the `NORMAL_DISPATCH_RATIO` that we adjust the fees with. Blocks filled less
	/// than this will decrease the weight and more will increase.
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	/// The adjustment variable of the runtime. Higher values will cause `TargetBlockFullness` to
	/// change the fees more rapidly.
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(75, 1000_000);
	/// Minimum amount of the multiplier. This value cannot be too low. A test case should ensure
	/// that combined with `AdjustmentVariable`, we can recover from the minimum.
	/// See `multiplier_can_grow_from_zero`.
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 10u128);
	/// The maximum amount of the multiplier.
	pub MaximumMultiplier: Multiplier = Bounded::max_value();
	/// Maximum length of block. Up to 5MB.
	pub BlockLength: limits::BlockLength =
	limits::BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
}

/// Parameterized slow adjusting fee updated based on
/// <https://research.web3.foundation/Polkadot/overview/token-economics#2-slow-adjusting-mechanism>
pub type SlowAdjustingFeeUpdate<R> = TargetedFeeAdjustment<
	R,
	TargetBlockFullness,
	AdjustmentVariable,
	MinimumMultiplier,
	MaximumMultiplier,
>;

/// Implements the weight types for a runtime.
/// It expects the passed runtime constants to contain a `weights` module.
/// The generated weight types were formerly part of the common
/// runtime but are now runtime dependant.
#[macro_export]
macro_rules! impl_runtime_weights {
	($runtime:ident) => {
		use pezframe_support::{dispatch::DispatchClass, weights::Weight};
		use pezframe_system::limits;
		pub use pezkuwi_runtime_common::{
			impl_elections_weights, AVERAGE_ON_INITIALIZE_RATIO, MAXIMUM_BLOCK_WEIGHT,
			NORMAL_DISPATCH_RATIO,
		};
		use pezpallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
		use pezsp_runtime::{FixedPointNumber, Perquintill};

		// Implement the weight types of the elections module.
		impl_elections_weights!($runtime);

		// Expose the weight from the runtime constants module.
		pub use $runtime::weights::{
			BlockExecutionWeight, ExtrinsicBaseWeight, ParityDbWeight, RocksDbWeight,
		};

		parameter_types! {
			/// Block weights base values and limits.
			pub BlockWeights: limits::BlockWeights = limits::BlockWeights::builder()
				.base_block($runtime::weights::BlockExecutionWeight::get())
				.for_class(DispatchClass::all(), |weights| {
					weights.base_extrinsic = $runtime::weights::ExtrinsicBaseWeight::get();
				})
				.for_class(DispatchClass::Normal, |weights| {
					weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
				})
				.for_class(DispatchClass::Operational, |weights| {
					weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
					// Operational transactions have an extra reserved space, so that they
					// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
					weights.reserved = Some(
						MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT,
					);
				})
				.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
				.build_or_panic();
		}
	};
}

/// The type used for currency conversion.
///
/// This must only be used as long as the balance type is `u128`.
pub type CurrencyToVote = pezsp_staking::currency_to_vote::U128CurrencyToVote;
static_assertions::assert_eq_size!(pezkuwi_primitives::Balance, u128);

/// A placeholder since there is currently no provided session key handler for teyrchain validator
/// keys.
pub struct TeyrchainSessionKeyPlaceholder<T>(core::marker::PhantomData<T>);
impl<T> pezsp_runtime::BoundToRuntimeAppPublic for TeyrchainSessionKeyPlaceholder<T> {
	type Public = ValidatorId;
}

impl<T: pezpallet_session::Config> OneSessionHandler<T::AccountId>
	for TeyrchainSessionKeyPlaceholder<T>
{
	type Key = ValidatorId;

	fn on_genesis_session<'a, I: 'a>(_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, ValidatorId)>,
		T::AccountId: 'a,
	{
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, _v: I, _q: I)
	where
		I: Iterator<Item = (&'a T::AccountId, ValidatorId)>,
		T::AccountId: 'a,
	{
	}

	fn on_disabled(_: u32) {}
}

/// A placeholder since there is currently no provided session key handler for teyrchain validator
/// keys.
pub struct AssignmentSessionKeyPlaceholder<T>(core::marker::PhantomData<T>);
impl<T> pezsp_runtime::BoundToRuntimeAppPublic for AssignmentSessionKeyPlaceholder<T> {
	type Public = AssignmentId;
}

impl<T: pezpallet_session::Config> OneSessionHandler<T::AccountId>
	for AssignmentSessionKeyPlaceholder<T>
{
	type Key = AssignmentId;

	fn on_genesis_session<'a, I: 'a>(_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, AssignmentId)>,
		T::AccountId: 'a,
	{
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, _v: I, _q: I)
	where
		I: Iterator<Item = (&'a T::AccountId, AssignmentId)>,
		T::AccountId: 'a,
	{
	}

	fn on_disabled(_: u32) {}
}

/// A reasonable benchmarking config for staking pezpallet.
pub struct StakingBenchmarkingConfig;
impl pezpallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxValidators = ConstU32<1000>;
	type MaxNominators = ConstU32<1000>;
}

/// Convert a balance to an unsigned 256-bit number, use in nomination pools.
pub struct BalanceToU256;
impl pezsp_runtime::traits::Convert<Balance, pezsp_core::U256> for BalanceToU256 {
	fn convert(n: Balance) -> pezsp_core::U256 {
		n.into()
	}
}

/// Convert an unsigned 256-bit number to balance, use in nomination pools.
pub struct U256ToBalance;
impl pezsp_runtime::traits::Convert<pezsp_core::U256, Balance> for U256ToBalance {
	fn convert(n: pezsp_core::U256) -> Balance {
		use pezframe_support::traits::Defensive;
		n.try_into().defensive_unwrap_or(Balance::MAX)
	}
}

/// Macro to set a value (e.g. when using the `parameter_types` macro) to either a production value
/// or to an environment variable or testing value (in case the `fast-runtime` feature is selected)
/// or one of two testing values depending on feature.
/// Note that the environment variable is evaluated _at compile time_.
///
/// Usage:
/// ```Rust
/// parameter_types! {
/// 	// Note that the env variable version parameter cannot be const.
/// 	pub LaunchPeriod: BlockNumber = prod_or_fast!(7 * DAYS, 1, "HEZ_LAUNCH_PERIOD");
/// 	pub const VotingPeriod: BlockNumber = prod_or_fast!(7 * DAYS, 1 * MINUTES);
/// 	pub const EpochDuration: BlockNumber =
/// 		prod_or_fast!(1 * HOURS, "fast-runtime", 1 * MINUTES, "fast-runtime-10m", 10 * MINUTES);
/// }
/// ```
#[macro_export]
macro_rules! prod_or_fast {
	($prod:expr, $test:expr) => {
		if cfg!(feature = "fast-runtime") {
			$test
		} else {
			$prod
		}
	};
	($prod:expr, $test:expr, $env:expr) => {
		if cfg!(feature = "fast-runtime") {
			core::option_env!($env).map(|s| s.parse().ok()).flatten().unwrap_or($test)
		} else {
			$prod
		}
	};
}
