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

//! Auxiliary struct/enums for teyrchain runtimes.
//! Taken from pezkuwi/runtime/common (at a21cd64) and adapted for teyrchains.

use alloc::boxed::Box;
use core::marker::PhantomData;
use pezframe_support::traits::{
	fungible, fungibles, tokens::imbalance::ResolveTo, Contains, ContainsPair, Currency, Defensive,
	Get, Imbalance, OnUnbalanced, OriginTrait,
};
use pezpallet_asset_tx_payment::HandleCredit;
use pezpallet_collator_selection::StakingPotAccountId;
use pezsp_runtime::traits::Zero;
use xcm::latest::{
	Asset, AssetId, Fungibility, Fungibility::Fungible, Junction, Junctions::Here, Location,
	Parent, WeightLimit,
};
use xcm_executor::traits::ConvertLocation;

/// Type alias to conveniently refer to `pezframe_system`'s `Config::AccountId`.
pub type AccountIdOf<T> = <T as pezframe_system::Config>::AccountId;

/// Type alias to conveniently refer to the `Currency::NegativeImbalance` associated type.
pub type NegativeImbalance<T> = <pezpallet_balances::Pezpallet<T> as Currency<
	<T as pezframe_system::Config>::AccountId,
>>::NegativeImbalance;

/// Implementation of `OnUnbalanced` that deposits the fees into a staking pot for later payout.
#[deprecated(
	note = "ToStakingPot is deprecated and will be removed after March 2024. Please use pezframe_support::traits::tokens::imbalance::ResolveTo instead."
)]
pub struct ToStakingPot<R>(PhantomData<R>);
#[allow(deprecated)]
impl<R> OnUnbalanced<NegativeImbalance<R>> for ToStakingPot<R>
where
	R: pezpallet_balances::Config + pezpallet_collator_selection::Config,
	AccountIdOf<R>: From<pezkuwi_primitives::AccountId> + Into<pezkuwi_primitives::AccountId>,
	<R as pezframe_system::Config>::RuntimeEvent: From<pezpallet_balances::Event<R>>,
{
	fn on_nonzero_unbalanced(amount: NegativeImbalance<R>) {
		let staking_pot = <pezpallet_collator_selection::Pezpallet<R>>::account_id();
		// In case of error: Will drop the result triggering the `OnDrop` of the imbalance.
		<pezpallet_balances::Pezpallet<R>>::resolve_creating(&staking_pot, amount);
	}
}

/// Fungible implementation of `OnUnbalanced` that deals with the fees by combining tip and fee and
/// passing the result on to `ToStakingPot`.
pub struct DealWithFees<R>(PhantomData<R>);
impl<R> OnUnbalanced<fungible::Credit<R::AccountId, pezpallet_balances::Pezpallet<R>>>
	for DealWithFees<R>
where
	R: pezpallet_balances::Config + pezpallet_collator_selection::Config,
	AccountIdOf<R>: From<pezkuwi_primitives::AccountId> + Into<pezkuwi_primitives::AccountId>,
	<R as pezframe_system::Config>::RuntimeEvent: From<pezpallet_balances::Event<R>>,
{
	fn on_unbalanceds(
		mut fees_then_tips: impl Iterator<
			Item = fungible::Credit<R::AccountId, pezpallet_balances::Pezpallet<R>>,
		>,
	) {
		if let Some(mut fees) = fees_then_tips.next() {
			if let Some(tips) = fees_then_tips.next() {
				tips.merge_into(&mut fees);
			}
			ResolveTo::<StakingPotAccountId<R>, pezpallet_balances::Pezpallet<R>>::on_unbalanced(
				fees,
			)
		}
	}
}

/// A `HandleCredit` implementation that naively transfers the fees to the block author.
/// Will drop and burn the assets in case the transfer fails.
pub struct AssetsToBlockAuthor<R, I>(PhantomData<(R, I)>);
impl<R, I> HandleCredit<AccountIdOf<R>, pezpallet_assets::Pezpallet<R, I>>
	for AssetsToBlockAuthor<R, I>
where
	I: 'static,
	R: pezpallet_authorship::Config + pezpallet_assets::Config<I>,
	AccountIdOf<R>: From<pezkuwi_primitives::AccountId> + Into<pezkuwi_primitives::AccountId>,
{
	fn handle_credit(credit: fungibles::Credit<AccountIdOf<R>, pezpallet_assets::Pezpallet<R, I>>) {
		use pezframe_support::traits::fungibles::Balanced;
		if let Some(author) = pezpallet_authorship::Pezpallet::<R>::author() {
			// In case of error: Will drop the result triggering the `OnDrop` of the imbalance.
			let _ = pezpallet_assets::Pezpallet::<R, I>::resolve(&author, credit).defensive();
		}
	}
}

/// Allow checking in assets that have issuance > 0.
pub struct NonZeroIssuance<AccountId, Assets>(PhantomData<(AccountId, Assets)>);
impl<AccountId, Assets> Contains<<Assets as fungibles::Inspect<AccountId>>::AssetId>
	for NonZeroIssuance<AccountId, Assets>
where
	Assets: fungibles::Inspect<AccountId>,
{
	fn contains(id: &<Assets as fungibles::Inspect<AccountId>>::AssetId) -> bool {
		!Assets::total_issuance(id.clone()).is_zero()
	}
}

/// Allow checking in assets that exists.
pub struct AssetExists<AccountId, Assets>(PhantomData<(AccountId, Assets)>);
impl<AccountId, Assets> Contains<<Assets as fungibles::Inspect<AccountId>>::AssetId>
	for AssetExists<AccountId, Assets>
where
	Assets: fungibles::Inspect<AccountId>,
{
	fn contains(id: &<Assets as fungibles::Inspect<AccountId>>::AssetId) -> bool {
		Assets::asset_exists(id.clone())
	}
}

/// Asset filter that allows all assets from a certain location.
pub struct AssetsFrom<T>(PhantomData<T>);
impl<T: Get<Location>> ContainsPair<Asset, Location> for AssetsFrom<T> {
	fn contains(asset: &Asset, origin: &Location) -> bool {
		let loc = T::get();
		&loc == origin
			&& matches!(asset, Asset { id: AssetId(asset_loc), fun: Fungible(_a) }
			if asset_loc.match_and_split(&loc).is_some())
	}
}

/// Type alias to conveniently refer to the `Currency::Balance` associated type.
pub type BalanceOf<T> = <pezpallet_balances::Pezpallet<T> as Currency<
	<T as pezframe_system::Config>::AccountId,
>>::Balance;

/// Implements `OnUnbalanced::on_unbalanced` to teleport slashed assets to relay chain treasury
/// account.
pub struct ToParentTreasury<TreasuryAccount, AccountIdConverter, T>(
	PhantomData<(TreasuryAccount, AccountIdConverter, T)>,
);

impl<TreasuryAccount, AccountIdConverter, T> OnUnbalanced<NegativeImbalance<T>>
	for ToParentTreasury<TreasuryAccount, AccountIdConverter, T>
where
	T: pezpallet_balances::Config + pezpallet_xcm::Config + pezframe_system::Config,
	<<T as pezframe_system::Config>::RuntimeOrigin as OriginTrait>::AccountId: From<AccountIdOf<T>>,
	[u8; 32]: From<<T as pezframe_system::Config>::AccountId>,
	TreasuryAccount: Get<AccountIdOf<T>>,
	AccountIdConverter: ConvertLocation<AccountIdOf<T>>,
	BalanceOf<T>: Into<Fungibility>,
{
	fn on_unbalanced(amount: NegativeImbalance<T>) {
		let amount = match amount.drop_zero() {
			Ok(..) => return,
			Err(amount) => amount,
		};
		let imbalance = amount.peek();
		let root_location: Location = Here.into();
		let root_account: AccountIdOf<T> =
			match AccountIdConverter::convert_location(&root_location) {
				Some(a) => a,
				None => {
					tracing::warn!(target: "xcm::on_unbalanced", "Failed to convert root origin into account id");
					return;
				},
			};
		let treasury_account: AccountIdOf<T> = TreasuryAccount::get();

		<pezpallet_balances::Pezpallet<T>>::resolve_creating(&root_account, amount);

		let result = <pezpallet_xcm::Pezpallet<T>>::limited_teleport_assets(
			<<T as pezframe_system::Config>::RuntimeOrigin>::root(),
			Box::new(Parent.into()),
			Box::new(
				Junction::AccountId32 { network: None, id: treasury_account.into() }
					.into_location()
					.into(),
			),
			Box::new((Parent, imbalance).into()),
			Box::new(Parent.into()),
			WeightLimit::Unlimited,
		);

		if let Err(err) = result {
			tracing::warn!(target: "xcm::on_unbalanced", error=?err, "Failed to teleport slashed assets");
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use pezframe_support::{
		derive_impl, parameter_types,
		traits::{ConstU32, FindAuthor, ValidatorRegistration},
		PalletId,
	};
	use pezframe_system::{limits, EnsureRoot};
	use pezkuwi_primitives::AccountId;
	use pezpallet_collator_selection::IdentityCollator;
	use pezsp_core::H256;
	use pezsp_runtime::{
		traits::{BlakeTwo256, IdentityLookup},
		BuildStorage, Perbill,
	};
	use xcm::prelude::*;

	type Block = pezframe_system::mocking::MockBlock<Test>;
	const TEST_ACCOUNT: AccountId = AccountId::new([1; 32]);

	pezframe_support::construct_runtime!(
		pub enum Test
		{
			System: pezframe_system::{Pezpallet, Call, Config<T>, Storage, Event<T>},
			Balances: pezpallet_balances::{Pezpallet, Call, Storage, Config<T>, Event<T>},
			CollatorSelection: pezpallet_collator_selection::{Pezpallet, Call, Storage, Event<T>},
		}
	);

	parameter_types! {
		pub BlockLength: limits::BlockLength = limits::BlockLength::max(2 * 1024);
		pub const AvailableBlockRatio: Perbill = Perbill::one();
	}

	#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
	impl pezframe_system::Config for Test {
		type BaseCallFilter = pezframe_support::traits::Everything;
		type RuntimeOrigin = RuntimeOrigin;
		type Nonce = u64;
		type RuntimeCall = RuntimeCall;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = AccountId;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Block = Block;
		type RuntimeEvent = RuntimeEvent;
		type BlockLength = BlockLength;
		type BlockWeights = ();
		type DbWeight = ();
		type Version = ();
		type PalletInfo = PalletInfo;
		type AccountData = pezpallet_balances::AccountData<u64>;
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type SystemWeightInfo = ();
		type SS58Prefix = ();
		type OnSetCode = ();
		type MaxConsumers = pezframe_support::traits::ConstU32<16>;
	}

	#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
	impl pezpallet_balances::Config for Test {
		type AccountStore = System;
	}

	pub struct OneAuthor;
	impl FindAuthor<AccountId> for OneAuthor {
		fn find_author<'a, I>(_: I) -> Option<AccountId>
		where
			I: 'a,
		{
			Some(TEST_ACCOUNT)
		}
	}

	pub struct IsRegistered;
	impl ValidatorRegistration<AccountId> for IsRegistered {
		fn is_registered(_id: &AccountId) -> bool {
			true
		}
	}

	parameter_types! {
		pub const PotId: PalletId = PalletId(*b"PotStake");
	}

	impl pezpallet_collator_selection::Config for Test {
		type RuntimeEvent = RuntimeEvent;
		type Currency = Balances;
		type UpdateOrigin = EnsureRoot<AccountId>;
		type PotId = PotId;
		type MaxCandidates = ConstU32<20>;
		type MinEligibleCollators = ConstU32<1>;
		type MaxInvulnerables = ConstU32<20>;
		type ValidatorId = <Self as pezframe_system::Config>::AccountId;
		type ValidatorIdOf = IdentityCollator;
		type ValidatorRegistration = IsRegistered;
		type KickThreshold = ();
		type WeightInfo = ();
	}

	impl pezpallet_authorship::Config for Test {
		type FindAuthor = OneAuthor;
		type EventHandler = ();
	}

	pub fn new_test_ext() -> pezsp_io::TestExternalities {
		let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		// We use default for brevity, but you can configure as desired if needed.
		pezpallet_balances::GenesisConfig::<Test>::default()
			.assimilate_storage(&mut t)
			.unwrap();
		t.into()
	}

	#[test]
	fn test_fees_and_tip_split() {
		new_test_ext().execute_with(|| {
			let fee =
				<pezpallet_balances::Pezpallet<Test> as pezframe_support::traits::fungible::Balanced<
					AccountId,
				>>::issue(10);
			let tip =
				<pezpallet_balances::Pezpallet<Test> as pezframe_support::traits::fungible::Balanced<
					AccountId,
				>>::issue(20);

			assert_eq!(Balances::free_balance(TEST_ACCOUNT), 0);

			DealWithFees::on_unbalanceds(vec![fee, tip].into_iter());

			// Author gets 100% of tip and 100% of fee = 30
			assert_eq!(Balances::free_balance(CollatorSelection::account_id()), 30);
		});
	}

	#[test]
	fn assets_from_filters_correctly() {
		parameter_types! {
			pub SomeSiblingTeyrchain: Location = (Parent, Teyrchain(1234)).into();
		}

		let asset_location = SomeSiblingTeyrchain::get()
			.pushed_with_interior(GeneralIndex(42))
			.expect("location will only have 2 junctions; qed");
		let asset = Asset { id: AssetId(asset_location), fun: 1_000_000u128.into() };
		assert!(
			AssetsFrom::<SomeSiblingTeyrchain>::contains(&asset, &SomeSiblingTeyrchain::get()),
			"AssetsFrom should allow assets from any of its interior locations"
		);
	}
}
