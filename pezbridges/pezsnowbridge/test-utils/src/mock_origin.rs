// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>

// A stripped-down version of pezpallet-xcm that only inserts an XCM origin into the runtime
#[pezframe_support::pezpallet]
pub mod pezpallet_xcm_origin {
	use codec::DecodeWithMemTracking;
	use pezframe_support::{
		pezpallet_prelude::*,
		traits::{Contains, OriginTrait},
	};
	use xcm::latest::prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		type RuntimeOrigin: From<Origin> + From<<Self as pezframe_system::Config>::RuntimeOrigin>;
	}

	// Insert this custom Origin into the aggregate RuntimeOrigin
	#[pezpallet::origin]
	#[derive(
		PartialEq,
		Eq,
		Clone,
		Encode,
		Decode,
		DecodeWithMemTracking,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct Origin(pub Location);

	impl From<Location> for Origin {
		fn from(location: Location) -> Origin {
			Origin(location)
		}
	}

	/// `EnsureOrigin` implementation succeeding with a `Location` value to recognize and
	/// filter the contained location
	pub struct EnsureXcm<F>(PhantomData<F>);
	impl<O: OriginTrait + From<Origin>, F: Contains<Location>> EnsureOrigin<O> for EnsureXcm<F>
	where
		O::PalletsOrigin: From<Origin> + TryInto<Origin, Error = O::PalletsOrigin>,
	{
		type Success = Location;

		fn try_origin(outer: O) -> Result<Self::Success, O> {
			outer.try_with_caller(|caller| {
				caller.try_into().and_then(|o| match o {
					Origin(location) if F::contains(&location) => Ok(location),
					o => Err(o.into()),
				})
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin(Location::here().into())))
		}
	}
}
