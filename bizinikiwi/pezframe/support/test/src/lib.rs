// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Minimal pezpallet without `pezframe_system::Config`-super trait.

// Make sure we fail compilation on warnings
#![warn(missing_docs)]
#![deny(warnings)]

pub use pezframe_support::dispatch::RawOrigin;
use pezframe_system::pezpallet_prelude::BlockNumberFor;

pub use self::pezpallet::*;

#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet {
	use super::*;
	use crate::{self as pezframe_system, pezpallet_prelude::*};
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	/// The configuration trait.
	#[pezpallet::config(pezframe_system_config)]
	#[pezpallet::disable_pezframe_system_supertrait_check]
	pub trait Config: 'static + Eq + Clone {
		/// The block number type.
		type BlockNumber: Parameter + Member + Default + MaybeSerializeDeserialize + MaxEncodedLen;
		/// The account type.
		type AccountId: Parameter + Member + MaxEncodedLen;
		/// The basic call filter to use in Origin.
		type BaseCallFilter: pezframe_support::traits::Contains<Self::RuntimeCall>;
		/// The runtime origin type.
		type RuntimeOrigin: Into<Result<RawOrigin<Self::AccountId>, Self::RuntimeOrigin>>
			+ From<RawOrigin<Self::AccountId>>;
		/// The runtime call type.
		type RuntimeCall;
		/// Contains an aggregation of all tasks in this runtime.
		type RuntimeTask;
		/// The runtime event type.
		type RuntimeEvent: Parameter
			+ Member
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>
			+ From<Event<Self>>;
		/// The information about the pezpallet setup in the runtime.
		type PalletInfo: pezframe_support::traits::PalletInfo;
		/// The db weights.
		type DbWeight: Get<pezframe_support::weights::RuntimeDbWeight>;
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// A noop call.
		pub fn noop(_origin: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		/// A empty method.
		pub fn deposit_event(_event: impl Into<T::RuntimeEvent>) {}
	}

	/// The origin type.
	#[pezpallet::origin]
	pub type Origin<T> = RawOrigin<<T as Config>::AccountId>;

	/// The error type.
	#[pezpallet::error]
	pub enum Error<T> {
		/// Test error documentation
		TestError,
		/// Error documentation
		/// with multiple lines
		AnotherError,
		/// Required by construct_runtime
		CallFiltered,
	}

	/// The event type.
	#[pezpallet::event]
	pub enum Event<T: Config> {
		/// The extrinsic is successful
		ExtrinsicSuccess,
		/// The extrinsic is failed
		ExtrinsicFailed,
		/// The ignored error
		Ignore(<T as Config>::BlockNumber),
	}
}

/// Ensure that the origin `o` represents the root. Returns `Ok` or an `Err` otherwise.
pub fn ensure_root<OuterOrigin, AccountId>(o: OuterOrigin) -> Result<(), &'static str>
where
	OuterOrigin: Into<Result<RawOrigin<AccountId>, OuterOrigin>>,
{
	o.into().map(|_| ()).map_err(|_| "bad origin: expected to be a root origin")
}

/// Same semantic as [`pezframe_system`].
// Note: we cannot use [`pezframe_system`] here since the pezpallet does not depend on
// [`pezframe_system::Config`].
pub mod pezpallet_prelude {
	pub use crate::ensure_root;

	/// Type alias for the `Origin` associated type of system config.
	pub type OriginFor<T> = <T as crate::Config>::RuntimeOrigin;

	/// Type alias for the `BlockNumber` associated type of system config.
	pub type BlockNumberFor<T> = <T as super::Config>::BlockNumber;
}

/// Provides an implementation of [`pezframe_support::traits::Randomness`] that should only be used
/// in tests!
pub struct TestRandomness<T>(core::marker::PhantomData<T>);

impl<Output: codec::Decode + Default, T>
	pezframe_support::traits::Randomness<Output, BlockNumberFor<T>> for TestRandomness<T>
where
	T: pezframe_system::Config,
{
	fn random(subject: &[u8]) -> (Output, BlockNumberFor<T>) {
		use pezsp_runtime::traits::TrailingZeroInput;

		(
			Output::decode(&mut TrailingZeroInput::new(subject)).unwrap_or_default(),
			pezframe_system::Pezpallet::<T>::block_number(),
		)
	}
}
