// Copyright (C) Dijital Kurdistan Tech Institute
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

//! The treasury pots' payment and spend-origin types.
//!
//! Both behave exactly like the types they wrap. They exist for their benchmark setup, which the
//! wrapped types get wrong for a pot on an Asset Hub, so the pots' weights could not be measured.

use core::marker::PhantomData;
use pezframe_support::traits::{
	tokens::{
		fungible,
		pay::{Pay, PayFromAccount, PaymentStatus},
	},
	EnsureOrigin, Equals, Get, OriginTrait, TypedGet,
};
use pezpallet_xcm::{EnsureXcm, Origin as XcmOrigin};
use xcm::latest::Location;

/// [`PayFromAccount`], whose benchmark setup also gives the beneficiary an existential deposit.
///
/// The treasury benchmarks spend a fixed 100 units. On a chain whose existential deposit is above
/// that, `PayFromAccount` cannot pay it to a fresh account, and its own setup mints those 100
/// units into the pot, which fails the same way if the pot is empty. Here the pot is minted the
/// amount plus the existential deposit and the beneficiary is made to exist first.
pub struct PayFromPot<F, A>(PhantomData<(F, A)>);
impl<A, F> Pay for PayFromPot<F, A>
where
	A: TypedGet,
	F: fungible::Mutate<A::Type>,
	A::Type: Eq,
{
	type Balance = F::Balance;
	type Beneficiary = A::Type;
	type AssetKind = ();
	type Id = ();
	type Error = pezsp_runtime::DispatchError;

	fn pay(
		who: &Self::Beneficiary,
		asset_kind: Self::AssetKind,
		amount: Self::Balance,
	) -> Result<Self::Id, Self::Error> {
		PayFromAccount::<F, A>::pay(who, asset_kind, amount)
	}

	fn check_payment(id: Self::Id) -> PaymentStatus {
		PayFromAccount::<F, A>::check_payment(id)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_successful(who: &Self::Beneficiary, _: Self::AssetKind, amount: Self::Balance) {
		use pezsp_runtime::Saturating;
		let existential_deposit = F::minimum_balance();
		F::mint_into(&A::get(), amount.saturating_add(existential_deposit))
			.expect("minting at least the existential deposit into the pot succeeds");
		F::mint_into(who, existential_deposit)
			.expect("minting the existential deposit into the beneficiary succeeds");
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_concluded(id: Self::Id) {
		PayFromAccount::<F, A>::ensure_concluded(id)
	}
}

/// An XCM origin from exactly the location `L`, succeeding with that location.
///
/// It accepts what `EnsureXcm<Equals<L>>` accepts. The difference is its benchmark origin:
/// `EnsureXcm` always offers `Here`, which `Equals<L>` rejects for any other `L`, so every
/// benchmark of a call guarded that way failed with `BadOrigin`. This one offers `L` itself.
pub struct EnsureXcmFrom<L>(PhantomData<L>);
impl<O, L> EnsureOrigin<O> for EnsureXcmFrom<L>
where
	O: OriginTrait + From<XcmOrigin>,
	for<'a> &'a O::PalletsOrigin: TryInto<&'a XcmOrigin>,
	L: Get<Location>,
{
	type Success = Location;

	fn try_origin(outer: O) -> Result<Self::Success, O> {
		EnsureXcm::<Equals<L>>::try_origin(outer)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<O, ()> {
		Ok(O::from(XcmOrigin::Xcm(L::get())))
	}
}
