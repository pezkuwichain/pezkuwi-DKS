// This file is part of Pezcumulus.

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

//! Tests for the Zagros Asset Hub chain.

extern crate alloc;

use alloy_core::{
	primitives::U256,
	sol_types::{sol_data, SolType},
};
use asset_hub_zagros_runtime::{
	xcm_config,
	xcm_config::{
		bridging, CheckingAccount, LocationToAccountId, StakingPot, TokenLocation,
		TrustBackedAssetsPalletLocation, UniquesConvertedConcreteId, UniquesPalletLocation,
		XcmConfig,
	},
	AllPalletsWithoutSystem, Assets, Balances, Block, ExistentialDeposit, ForeignAssets,
	ForeignAssetsInstance, MetadataDepositBase, MetadataDepositPerByte, PezkuwiXcm, Revive,
	Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, SessionKeys, TeyrchainSystem,
	ToPezkuwichainXcmRouterInstance, TrustBackedAssetsInstance, Uniques, WeightToFee, XcmpQueue,
};
pub use asset_hub_zagros_runtime::{AssetConversion, AssetDeposit, CollatorSelection, System};
use asset_test_pezutils::{
	test_cases_over_bridge::TestBridgingConfig, CollatorSessionKey, CollatorSessionKeys,
	ExtBuilder, GovernanceOrigin, SlotDurations,
};
use codec::{Decode, Encode};
use hex_literal::hex;
use pez_assets_common::local_and_foreign_assets::ForeignAssetReserveData;
use pezframe_support::{
	assert_err, assert_noop, assert_ok, parameter_types,
	traits::{
		fungible::{self, Inspect, Mutate},
		fungibles::{
			self, Create, Inspect as FungiblesInspect, InspectEnumerable, Mutate as FungiblesMutate,
		},
		tokens::asset_ops::{
			common_strategies::{Bytes, Owner},
			Inspect as InspectUniqueAsset,
		},
		ContainsPair,
	},
	weights::{Weight, WeightToFee as WeightToFeeT},
};
use pezpallet_revive::{
	test_utils::builder::{BareInstantiateBuilder, Contract},
	Code, TransactionLimits,
};
use pezpallet_revive_fixtures::compile_module;
use pezpallet_uniques::{asset_ops::Item, asset_strategies::Attribute};
use pezsp_consensus_aura::SlotDuration;
use pezsp_core::crypto::Ss58Codec;
use pezsp_runtime::{traits::MaybeEquivalence, Either, MultiAddress};
use std::convert::Into;
use testnet_teyrchains_constants::zagros::{consensus::*, currency::UNITS};
use teyrchains_common::{AccountId, AssetIdForTrustBackedAssets, AuraId, Balance};
use xcm::{
	latest::{
		prelude::{Assets as XcmAssets, *},
		PEZKUWICHAIN_GENESIS_HASH,
	},
	VersionedXcm,
};
use xcm_builder::{
	unique_instances::UniqueInstancesAdapter as NewNftAdapter, MatchInClassInstances, NoChecking,
	NonFungiblesAdapter as OldNftAdapter, WithLatestLocationConverter,
};
use xcm_executor::traits::{ConvertLocation, JustTry, TransactAsset, WeightTrader};
use xcm_runtime_pezapis::conversions::LocationToAccountHelper;

const ALICE: [u8; 32] = [1u8; 32];
const BOB: [u8; 32] = [2u8; 32];
const SOME_ASSET_ADMIN: [u8; 32] = [5u8; 32];

const ERC20_PVM: &[u8] =
	include_bytes!("../../../../../../bizinikiwi/pezframe/revive/fixtures/erc20/erc20.polkavm");

const FAKE_ERC20_PVM: &[u8] = include_bytes!(
	"../../../../../../bizinikiwi/pezframe/revive/fixtures/erc20/fake_erc20.polkavm"
);

const EXPENSIVE_ERC20_PVM: &[u8] = include_bytes!(
	"../../../../../../bizinikiwi/pezframe/revive/fixtures/erc20/expensive_erc20.polkavm"
);

parameter_types! {
	pub Governance: GovernanceOrigin<RuntimeOrigin> = GovernanceOrigin::Origin(RuntimeOrigin::root());
}

type AssetIdForTrustBackedAssetsConvert =
	pez_assets_common::AssetIdForTrustBackedAssetsConvert<TrustBackedAssetsPalletLocation>;

type RuntimeHelper = asset_test_pezutils::RuntimeHelper<Runtime, AllPalletsWithoutSystem>;

fn collator_session_key(account: [u8; 32]) -> CollatorSessionKey<Runtime> {
	CollatorSessionKey::new(
		AccountId::from(account),
		AccountId::from(account),
		SessionKeys { aura: AuraId::from(pezsp_core::sr25519::Public::from_raw(account)) },
	)
}

fn collator_session_keys() -> CollatorSessionKeys<Runtime> {
	CollatorSessionKeys::default().add(collator_session_key(ALICE))
}

fn slot_durations() -> SlotDurations {
	SlotDurations {
		relay: SlotDuration::from_millis(RELAY_CHAIN_SLOT_DURATION_MILLIS.into()),
		para: SlotDuration::from_millis(SLOT_DURATION),
	}
}

/// Build a bare_instantiate call.
fn bare_instantiate(origin: &AccountId, code: Vec<u8>) -> BareInstantiateBuilder<Runtime> {
	let origin = RuntimeOrigin::signed(origin.clone());
	BareInstantiateBuilder::<Runtime>::bare_instantiate(origin, Code::Upload(code))
}

#[test]
fn test_buy_and_refund_weight_in_native() {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(pezsp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let bob: AccountId = SOME_ASSET_ADMIN.into();
			let staking_pot = CollatorSelection::account_id();
			let native_location = TokenLocation::get();
			let initial_balance = 200 * UNITS;

			assert_ok!(Balances::mint_into(&bob, initial_balance));
			assert_ok!(Balances::mint_into(&staking_pot, initial_balance));

			// keep initial total issuance to assert later.
			let total_issuance = Balances::total_issuance();

			// prepare input to buy weight.
			let weight = Weight::from_parts(4_000_000_000, 0);
			let fee = WeightToFee::weight_to_fee(&weight);
			let extra_amount = 100;
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };
			let payment: Asset = (native_location.clone(), fee + extra_amount).into();

			// AssetsInHolding no longer converts from an Asset: it carries imbalances now, so the
			// payment has to come out of a real account.
			let bob_location: Location =
				Junction::AccountId32 { network: None, id: bob.clone().into() }.into();
			let payment_holding =
				<XcmConfig as xcm_executor::Config>::AssetTransactor::withdraw_asset(
					&payment,
					&bob_location,
					Some(&ctx),
				)
				.expect("Failed to withdraw payment");

			// init trader and buy weight.
			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let unused_asset =
				trader.buy_weight(weight, payment_holding, &ctx).expect("Expected Ok");

			// assert.
			let unused_amount = unused_asset
				.fungible
				.get(&native_location.clone().into())
				.map_or(0, |a| a.amount());
			assert_eq!(unused_amount, extra_amount);
			assert_eq!(Balances::total_issuance(), total_issuance);

			// prepare input to refund weight.
			let refund_weight = Weight::from_parts(1_000_000_000, 0);
			let refund = WeightToFee::weight_to_fee(&refund_weight);

			// refund.
			let actual_refund = trader.refund_weight(refund_weight, &ctx).unwrap();
			let actual_refund_amount = actual_refund
				.fungible
				.get(&native_location.clone().into())
				.map_or(0, |a| a.amount());
			assert_eq!(actual_refund_amount, refund);

			// assert.
			assert_eq!(Balances::balance(&staking_pot), initial_balance);
			// only after `trader` is dropped we expect the fee to be resolved into the treasury
			// account.
			drop(trader);
			assert_eq!(Balances::balance(&staking_pot), initial_balance + fee - refund);
			// Unchanged, not raised by the fee. The fee is *moved* -- withdrawn from the payer
			// and deposited to the pot -- so nothing is created by charging it. The old
			// expectation came from the holding model this file was half-migrated away from,
			// where a trader's fee appeared out of the accounting rather than out of an
			// account, and a fee that inflates the supply is the last thing a fixed-supply
			// chain should assert as correct.
			assert_eq!(Balances::total_issuance(), total_issuance);
		})
}

#[test]
fn test_buy_and_refund_weight_with_swap_local_asset_xcm_trader() {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(pezsp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let bob: AccountId = SOME_ASSET_ADMIN.into();
			let staking_pot = CollatorSelection::account_id();
			let asset_1: u32 = 1;
			let native_location = TokenLocation::get();
			let asset_1_location =
				AssetIdForTrustBackedAssetsConvert::convert_back(&asset_1).unwrap();
			// bob's initial balance for native and `asset1` assets.
			let initial_balance = 200 * UNITS;
			// liquidity for both arms of (native, asset1) pool.
			let pool_liquidity = 100 * UNITS;

			// init asset, balances and pool.
			assert_ok!(<Assets as Create<_>>::create(asset_1, bob.clone(), true, 10));

			assert_ok!(Assets::mint_into(asset_1, &bob, initial_balance));
			assert_ok!(Balances::mint_into(&bob, initial_balance));
			assert_ok!(Balances::mint_into(&staking_pot, initial_balance));

			assert_ok!(AssetConversion::create_pool(
				RuntimeHelper::origin_of(bob.clone()),
				Box::new(
					xcm::v5::Location::try_from(native_location.clone()).expect("conversion works")
				),
				Box::new(
					xcm::v5::Location::try_from(asset_1_location.clone())
						.expect("conversion works")
				)
			));

			assert_ok!(AssetConversion::add_liquidity(
				RuntimeHelper::origin_of(bob.clone()),
				Box::new(
					xcm::v5::Location::try_from(native_location.clone()).expect("conversion works")
				),
				Box::new(
					xcm::v5::Location::try_from(asset_1_location.clone())
						.expect("conversion works")
				),
				pool_liquidity,
				pool_liquidity,
				1,
				1,
				bob.clone(),
			));

			// keep initial total issuance to assert later.
			let asset_total_issuance = Assets::total_issuance(asset_1);
			let native_total_issuance = Balances::total_issuance();

			// prepare input to buy weight.
			let weight = Weight::from_parts(4_000_000_000, 0);
			let fee = WeightToFee::weight_to_fee(&weight);
			let asset_fee =
				AssetConversion::get_amount_in(&fee, &pool_liquidity, &pool_liquidity).unwrap();
			let extra_amount = 100;
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };
			let payment: Asset = (asset_1_location.clone(), asset_fee + extra_amount).into();

			// AssetsInHolding no longer converts from an Asset: it carries imbalances now, so the
			// payment has to come out of a real account.
			let bob_location: Location =
				Junction::AccountId32 { network: None, id: bob.clone().into() }.into();
			let payment_holding =
				<XcmConfig as xcm_executor::Config>::AssetTransactor::withdraw_asset(
					&payment,
					&bob_location,
					Some(&ctx),
				)
				.expect("Failed to withdraw payment");

			// init trader and buy weight.
			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let unused_asset =
				trader.buy_weight(weight, payment_holding, &ctx).expect("Expected Ok");

			// assert.
			let unused_amount = unused_asset
				.fungible
				.get(&asset_1_location.clone().into())
				.map_or(0, |a| a.amount());
			assert_eq!(unused_amount, extra_amount);
			// Unchanged: the fee is taken from the payer and swapped, not minted. See the
			// note on the native trader above -- the `+ fee` expectations are leftovers from
			// the holding model this file was half-migrated away from.
			assert_eq!(Assets::total_issuance(asset_1), asset_total_issuance);

			// prepare input to refund weight.
			let refund_weight = Weight::from_parts(1_000_000_000, 0);
			let refund = WeightToFee::weight_to_fee(&refund_weight);
			let (reserve1, reserve2) = AssetConversion::get_reserves(
				xcm::v5::Location::try_from(native_location).expect("conversion works"),
				xcm::v5::Location::try_from(asset_1_location.clone()).expect("conversion works"),
			)
			.unwrap();
			let asset_refund =
				AssetConversion::get_amount_out(&refund, &reserve1, &reserve2).unwrap();

			// refund.
			let actual_refund = trader.refund_weight(refund_weight, &ctx).unwrap();
			let actual_refund_amount = actual_refund
				.fungible
				.get(&asset_1_location.clone().into())
				.map_or(0, |a| a.amount());
			assert_eq!(actual_refund_amount, asset_refund);

			// assert.
			assert_eq!(Balances::balance(&staking_pot), initial_balance);
			// only after `trader` is dropped we expect the fee to be resolved into the treasury
			// account.
			drop(trader);
			assert_eq!(Balances::balance(&staking_pot), initial_balance + fee - refund);
			// Unchanged: the fee is taken from the payer and swapped, not minted. See the
			// note on the native trader above -- the `+ fee` expectations are leftovers from
			// the holding model this file was half-migrated away from.
			assert_eq!(Assets::total_issuance(asset_1), asset_total_issuance);
			assert_eq!(Balances::total_issuance(), native_total_issuance);
		})
}

#[test]
fn test_buy_and_refund_weight_with_swap_foreign_asset_xcm_trader() {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(pezsp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let bob: AccountId = SOME_ASSET_ADMIN.into();
			let staking_pot = CollatorSelection::account_id();
			let native_location =
				xcm::v5::Location::try_from(TokenLocation::get()).expect("conversion works");
			let foreign_location = xcm::v5::Location {
				parents: 1,
				interior: (
					xcm::v5::Junction::Teyrchain(1234),
					xcm::v5::Junction::GeneralIndex(12345),
				)
					.into(),
			};
			// bob's initial balance for native and `asset1` assets.
			let initial_balance = 200 * UNITS;
			// liquidity for both arms of (native, asset1) pool.
			let pool_liquidity = 100 * UNITS;

			// init asset, balances and pool.
			assert_ok!(<ForeignAssets as Create<_>>::create(
				foreign_location.clone(),
				bob.clone(),
				true,
				10
			));

			assert_ok!(ForeignAssets::mint_into(foreign_location.clone(), &bob, initial_balance));
			assert_ok!(Balances::mint_into(&bob, initial_balance));
			assert_ok!(Balances::mint_into(&staking_pot, initial_balance));

			assert_ok!(AssetConversion::create_pool(
				RuntimeHelper::origin_of(bob.clone()),
				Box::new(native_location.clone()),
				Box::new(foreign_location.clone())
			));

			assert_ok!(AssetConversion::add_liquidity(
				RuntimeHelper::origin_of(bob.clone()),
				Box::new(native_location.clone()),
				Box::new(foreign_location.clone()),
				pool_liquidity,
				pool_liquidity,
				1,
				1,
				bob.clone(),
			));

			// keep initial total issuance to assert later.
			let asset_total_issuance = ForeignAssets::total_issuance(foreign_location.clone());
			let native_total_issuance = Balances::total_issuance();

			// prepare input to buy weight.
			let weight = Weight::from_parts(4_000_000_000, 0);
			let fee = WeightToFee::weight_to_fee(&weight);
			let asset_fee =
				AssetConversion::get_amount_in(&fee, &pool_liquidity, &pool_liquidity).unwrap();
			let extra_amount = 100;
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };
			let payment: Asset = (foreign_location.clone(), asset_fee + extra_amount).into();

			// AssetsInHolding no longer converts from an Asset: it carries imbalances now, so the
			// payment has to come out of a real account.
			let bob_location: Location =
				Junction::AccountId32 { network: None, id: bob.clone().into() }.into();
			let payment_holding =
				<XcmConfig as xcm_executor::Config>::AssetTransactor::withdraw_asset(
					&payment,
					&bob_location,
					Some(&ctx),
				)
				.expect("Failed to withdraw payment");

			// init trader and buy weight.
			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let unused_asset =
				trader.buy_weight(weight, payment_holding, &ctx).expect("Expected Ok");

			// assert.
			let unused_amount = unused_asset
				.fungible
				.get(&foreign_location.clone().into())
				.map_or(0, |a| a.amount());
			assert_eq!(unused_amount, extra_amount);
			// Unchanged: the fee is taken from the payer and swapped, not minted. See the
			// note on the native trader above -- the `+ fee` expectations are leftovers from
			// the holding model this file was half-migrated away from.
			assert_eq!(
				ForeignAssets::total_issuance(foreign_location.clone()),
				asset_total_issuance
			);

			// prepare input to refund weight.
			let refund_weight = Weight::from_parts(1_000_000_000, 0);
			let refund = WeightToFee::weight_to_fee(&refund_weight);
			let (reserve1, reserve2) =
				AssetConversion::get_reserves(native_location, foreign_location.clone()).unwrap();
			let asset_refund =
				AssetConversion::get_amount_out(&refund, &reserve1, &reserve2).unwrap();

			// refund.
			let actual_refund = trader.refund_weight(refund_weight, &ctx).unwrap();
			let actual_refund_amount = actual_refund
				.fungible
				.get(&foreign_location.clone().into())
				.map_or(0, |a| a.amount());
			assert_eq!(actual_refund_amount, asset_refund);

			// assert.
			assert_eq!(Balances::balance(&staking_pot), initial_balance);
			// only after `trader` is dropped we expect the fee to be resolved into the treasury
			// account.
			drop(trader);
			assert_eq!(Balances::balance(&staking_pot), initial_balance + fee - refund);
			// Unchanged, for the same reason as every other issuance assertion here: the fee
			// moves, it is not created.
			assert_eq!(ForeignAssets::total_issuance(foreign_location), asset_total_issuance);
			assert_eq!(Balances::total_issuance(), native_total_issuance);
		})
}

#[test]
fn test_asset_xcm_take_first_trader_refund_not_possible_since_amount_less_than_ed() {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(pezsp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// We need root origin to create a sufficient asset
			// We set existential deposit to be identical to the one for Balances first
			assert_ok!(Assets::force_create(
				RuntimeHelper::root_origin(),
				1.into(),
				AccountId::from(ALICE).into(),
				true,
				ExistentialDeposit::get()
			));

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };

			// Set Alice as block author, who will receive fees
			RuntimeHelper::run_to_block(2, AccountId::from(ALICE));

			// We are going to buy small amount
			let bought = Weight::from_parts(500_000_000u64, 0);

			let asset_location = AssetIdForTrustBackedAssetsConvert::convert_back(&1).unwrap();

			let amount_bought = WeightToFee::weight_to_fee(&bought);

			assert!(
				amount_bought < ExistentialDeposit::get(),
				"we are testing what happens when the amount does not exceed ED"
			);

			let asset: Asset = (asset_location.clone(), amount_bought).into();

			// Buy weight should return an error
			// The holding is built by withdrawing, so the account needs a balance to withdraw
			// from -- at least ED, or the mint itself is rejected.
			let mint_amount = amount_bought.max(ExistentialDeposit::get() + 1);
			assert_ok!(Assets::mint(
				RuntimeHelper::origin_of(AccountId::from(ALICE)),
				1.into(),
				AccountId::from(ALICE).into(),
				mint_amount
			));
			let alice_location: Location =
				Junction::AccountId32 { network: None, id: ALICE.into() }.into();
			let asset_holding =
				<XcmConfig as xcm_executor::Config>::AssetTransactor::withdraw_asset(
					&asset,
					&alice_location,
					Some(&ctx),
				)
				.expect("Failed to withdraw asset");

			// Buy weight fails and hands the asset back inside the error.
			let result = trader.buy_weight(bought, asset_holding, &ctx);
			assert!(result.is_err());
			if let Err((returned_asset, xcm_error)) = result {
				assert_eq!(xcm_error, XcmError::TooExpensive);
				// The whole minted amount comes back: withdrawing only `amount_bought` would
				// leave a sub-ED remainder, so the transactor takes the account down.
				assert_eq!(
					returned_asset.fungible.get(&asset_location.into()).map_or(0, |a| a.amount()),
					mint_amount
				);
			}

			// not credited since the ED is higher than this value
			assert_eq!(Assets::balance(1, AccountId::from(ALICE)), 0);

			// We also need to ensure the total supply did not increase
			assert_eq!(Assets::total_supply(1), 0);
		});
}

#[test]
fn test_asset_xcm_take_first_trader_not_possible_for_non_sufficient_assets() {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(pezsp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// Create a non-sufficient asset with specific existential deposit
			let minimum_asset_balance = 1_000_000_u128;
			assert_ok!(Assets::force_create(
				RuntimeHelper::root_origin(),
				1.into(),
				AccountId::from(ALICE).into(),
				false,
				minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(Assets::mint(
				RuntimeHelper::origin_of(AccountId::from(ALICE)),
				1.into(),
				AccountId::from(ALICE).into(),
				minimum_asset_balance
			));

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };

			// Set Alice as block author, who will receive fees
			RuntimeHelper::run_to_block(2, AccountId::from(ALICE));

			// We are going to buy 4e9 weight
			let bought = Weight::from_parts(4_000_000_000u64, 0);

			// lets calculate amount needed
			let asset_amount_needed = WeightToFee::weight_to_fee(&bought);

			let asset_location = AssetIdForTrustBackedAssetsConvert::convert_back(&1).unwrap();

			let asset: Asset = (asset_location.clone(), asset_amount_needed).into();

			// Mint what the withdraw below takes; alice only holds the minimum balance.
			assert_ok!(Assets::mint(
				RuntimeHelper::origin_of(AccountId::from(ALICE)),
				1.into(),
				AccountId::from(ALICE).into(),
				asset_amount_needed
			));

			// Make sure again buy_weight does return an error, handing the asset back.
			let alice_location: Location =
				Junction::AccountId32 { network: None, id: ALICE.into() }.into();
			let asset_holding =
				<XcmConfig as xcm_executor::Config>::AssetTransactor::withdraw_asset(
					&asset,
					&alice_location,
					Some(&ctx),
				)
				.expect("Failed to withdraw asset");
			let result = trader.buy_weight(bought, asset_holding, &ctx);
			assert!(result.is_err());
			if let Err((returned_asset, xcm_error)) = result {
				assert_eq!(xcm_error, XcmError::TooExpensive);
				assert_eq!(
					returned_asset.fungible.get(&asset_location.into()).map_or(0, |a| a.amount()),
					asset_amount_needed
				);
			}

			// Drop trader
			drop(trader);

			// Make sure author(Alice) has NOT received the amount
			assert_eq!(Assets::balance(1, AccountId::from(ALICE)), minimum_asset_balance);

			// We also need to ensure the total supply NOT increased
			assert_eq!(Assets::total_supply(1), minimum_asset_balance);
		});
}

fn test_nft_asset_transactor_works<T: TransactAsset>() {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(pezsp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let collection_id = 42;
			let item_id = 101;

			let alice = AccountId::from(ALICE);
			let bob = AccountId::from(BOB);
			let ctx = XcmContext { origin: None, message_id: XcmHash::default(), topic: None };

			assert_ok!(Balances::mint_into(&alice, 2 * UNITS));

			assert_ok!(Uniques::create(
				RuntimeHelper::origin_of(alice.clone()),
				collection_id,
				MultiAddress::Id(alice.clone()),
			));

			assert_ok!(Uniques::mint(
				RuntimeHelper::origin_of(alice.clone()),
				collection_id,
				item_id,
				MultiAddress::Id(bob.clone()),
			));

			let attr_key = vec![0xA, 0xA, 0xB, 0xB];
			let attr_value = vec![0xC, 0x0, 0x0, 0x1, 0xF, 0x0, 0x0, 0xD];

			assert_ok!(Uniques::set_attribute(
				RuntimeHelper::origin_of(alice.clone()),
				collection_id,
				Some(item_id),
				attr_key.clone().try_into().unwrap(),
				attr_value.clone().try_into().unwrap(),
			));

			let collection_location = UniquesPalletLocation::get()
				.appended_with(GeneralIndex(collection_id.into()))
				.unwrap();
			let item_asset: Asset =
				(collection_location.clone(), AssetInstance::Index(item_id.into())).into();

			let alice_account_location: Location = alice.clone().into();
			let bob_account_location: Location = bob.clone().into();

			// Can't deposit the token that isn't withdrawn
			let item_holding = xcm_executor::AssetsInHolding::new_from_non_fungible(
				collection_location.clone().into(),
				AssetInstance::Index(item_id.into()),
			);
			assert!(matches!(
				T::deposit_asset(item_holding, &alice_account_location, Some(&ctx)),
				Err((_, XcmError::FailedToTransactAsset("AlreadyExists")))
			));

			// Alice isn't the owner, she can't withdraw the token
			assert_noop!(
				T::withdraw_asset(&item_asset, &alice_account_location, Some(&ctx),),
				XcmError::FailedToTransactAsset("NoPermission")
			);

			// Bob, the owner, can withdraw the token. The holding it returns is what the
			// deposit below consumes -- deposit_asset takes a holding, not an asset.
			let withdrawn_holding =
				T::withdraw_asset(&item_asset, &bob_account_location, Some(&ctx))
					.expect("Bob owns the token");

			// The token is withdrawn
			assert_eq!(
				Item::<Uniques>::inspect(&(collection_id, item_id), Owner::default()),
				Err(pezpallet_uniques::Error::<Runtime>::UnknownItem.into()),
			);

			// But the attribute data is preserved as the pezpallet-uniques works that way.
			assert_eq!(
				Item::<Uniques>::inspect(
					&(collection_id, item_id),
					Bytes(Attribute(attr_key.as_slice()))
				),
				Ok(attr_value.clone()),
			);

			// Can't withdraw the already withdrawn token
			assert_err!(
				T::withdraw_asset(&item_asset, &bob_account_location, Some(&ctx),),
				XcmError::FailedToTransactAsset("UnknownCollection")
			);

			// Deposit the token to alice, using the holding bob's withdraw produced
			assert_ok!(T::deposit_asset(withdrawn_holding, &alice_account_location, Some(&ctx),));

			// The token is deposited
			assert_eq!(
				Item::<Uniques>::inspect(&(collection_id, item_id), Owner::default()),
				Ok(alice.clone()),
			);

			// The attribute data is the same
			assert_eq!(
				Item::<Uniques>::inspect(
					&(collection_id, item_id),
					Bytes(Attribute(attr_key.as_slice()))
				),
				Ok(attr_value.clone()),
			);

			// Can't deposit the token twice
			let item_holding_again = xcm_executor::AssetsInHolding::new_from_non_fungible(
				collection_location.clone().into(),
				AssetInstance::Index(item_id.into()),
			);
			assert!(matches!(
				T::deposit_asset(item_holding_again, &alice_account_location, Some(&ctx)),
				Err((_, XcmError::FailedToTransactAsset("AlreadyExists")))
			));

			// Transfer the token directly
			assert_ok!(T::transfer_asset(
				&item_asset,
				&alice_account_location,
				&bob_account_location,
				&ctx,
			));

			// The token's owner has changed
			assert_eq!(
				Item::<Uniques>::inspect(&(collection_id, item_id), Owner::default()),
				Ok(bob.clone()),
			);

			// The attribute data is the same
			assert_eq!(
				Item::<Uniques>::inspect(
					&(collection_id, item_id),
					Bytes(Attribute(attr_key.as_slice()))
				),
				Ok(attr_value.clone()),
			);
		});
}

#[test]
fn test_new_nft_config_works_as_the_old_one() {
	type OldNftTransactor = OldNftAdapter<
		Uniques,
		UniquesConvertedConcreteId,
		LocationToAccountId,
		AccountId,
		NoChecking,
		CheckingAccount,
	>;

	type NewNftTransactor = NewNftAdapter<
		AccountId,
		LocationToAccountId,
		MatchInClassInstances<UniquesConvertedConcreteId>,
		Item<Uniques>,
	>;

	test_nft_asset_transactor_works::<OldNftTransactor>();
	test_nft_asset_transactor_works::<NewNftTransactor>();
}

#[test]
fn test_assets_balances_api_works() {
	use pez_assets_common::runtime_api::runtime_decl_for_fungibles_api::FungiblesApi;

	ExtBuilder::<Runtime>::default()
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(pezsp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let local_asset_id = 1;
			let foreign_asset_id_location = xcm::v5::Location {
				parents: 1,
				interior: [
					xcm::v5::Junction::Teyrchain(1234),
					xcm::v5::Junction::GeneralIndex(12345),
				]
				.into(),
			};

			// check before
			assert_eq!(Assets::balance(local_asset_id, AccountId::from(ALICE)), 0);
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_location.clone(), AccountId::from(ALICE)),
				0
			);
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), 0);
			assert!(Runtime::query_account_balances(AccountId::from(ALICE))
				.unwrap()
				.try_as::<XcmAssets>()
				.unwrap()
				.is_none());

			// Drip some balance
			use pezframe_support::traits::fungible::Mutate;
			let some_currency = ExistentialDeposit::get();
			Balances::mint_into(&AccountId::from(ALICE), some_currency).unwrap();

			// We need root origin to create a sufficient asset
			let minimum_asset_balance = 3333333_u128;
			assert_ok!(Assets::force_create(
				RuntimeHelper::root_origin(),
				local_asset_id.into(),
				AccountId::from(ALICE).into(),
				true,
				minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(Assets::mint(
				RuntimeHelper::origin_of(AccountId::from(ALICE)),
				local_asset_id.into(),
				AccountId::from(ALICE).into(),
				minimum_asset_balance
			));

			// create foreign asset
			let foreign_asset_minimum_asset_balance = 3333333_u128;
			assert_ok!(ForeignAssets::force_create(
				RuntimeHelper::root_origin(),
				foreign_asset_id_location.clone(),
				AccountId::from(SOME_ASSET_ADMIN).into(),
				false,
				foreign_asset_minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(ForeignAssets::mint(
				RuntimeHelper::origin_of(AccountId::from(SOME_ASSET_ADMIN)),
				foreign_asset_id_location.clone(),
				AccountId::from(ALICE).into(),
				6 * foreign_asset_minimum_asset_balance
			));

			// check after
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(ALICE)),
				minimum_asset_balance
			);
			assert_eq!(
				ForeignAssets::balance(foreign_asset_id_location.clone(), AccountId::from(ALICE)),
				6 * minimum_asset_balance
			);
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), some_currency);

			let result: XcmAssets = Runtime::query_account_balances(AccountId::from(ALICE))
				.unwrap()
				.try_into()
				.unwrap();
			assert_eq!(result.len(), 3);

			// check currency
			assert!(result.inner().iter().any(|asset| {
				asset.eq(&pez_assets_common::fungible_conversion::convert_balance::<
					TokenLocation,
					Balance,
				>(some_currency)
				.unwrap())
			}));
			// check trusted asset
			assert!(result.inner().iter().any(|asset| asset.eq(&(
				AssetIdForTrustBackedAssetsConvert::convert_back(&local_asset_id).unwrap(),
				minimum_asset_balance
			)
				.into())));
			// check foreign asset
			assert!(result.inner().iter().any(|asset| asset.eq(&(
				WithLatestLocationConverter::<xcm::v5::Location>::convert_back(
					&foreign_asset_id_location
				)
				.unwrap(),
				6 * foreign_asset_minimum_asset_balance
			)
				.into())));
		});
}

#[test]
fn authorized_aliases_work() {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(pezsp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let alice: AccountId = ALICE.into();
			let local_alice = Location::new(0, AccountId32 { network: None, id: ALICE });
			let alice_on_sibling_para =
				Location::new(1, [Teyrchain(42), AccountId32 { network: None, id: ALICE }]);
			let alice_on_relay = Location::new(1, AccountId32 { network: None, id: ALICE });
			let bob_on_relay = Location::new(1, AccountId32 { network: None, id: [42_u8; 32] });

			assert_ok!(Balances::mint_into(&alice, 2 * UNITS));

			// neither `alice_on_sibling_para`, `alice_on_relay`, `bob_on_relay` are allowed to
			// alias into `local_alice`
			for aliaser in [&alice_on_sibling_para, &alice_on_relay, &bob_on_relay] {
				assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
					aliaser,
					&local_alice
				));
			}

			// Alice explicitly authorizes `alice_on_sibling_para` to alias her local account
			assert_ok!(PezkuwiXcm::add_authorized_alias(
				RuntimeHelper::origin_of(alice.clone()),
				Box::new(alice_on_sibling_para.clone().into()),
				None
			));

			// `alice_on_sibling_para` now explicitly allowed to alias into `local_alice`
			assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(
				&alice_on_sibling_para,
				&local_alice
			));
			// as expected, `alice_on_relay` and `bob_on_relay` still can't alias into `local_alice`
			for aliaser in [&alice_on_relay, &bob_on_relay] {
				assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
					aliaser,
					&local_alice
				));
			}

			// Alice explicitly authorizes `alice_on_relay` to alias her local account
			assert_ok!(PezkuwiXcm::add_authorized_alias(
				RuntimeHelper::origin_of(alice.clone()),
				Box::new(alice_on_relay.clone().into()),
				None
			));
			// Now both `alice_on_relay` and `alice_on_sibling_para` can alias into her local
			// account
			for aliaser in [&alice_on_relay, &alice_on_sibling_para] {
				assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(
					aliaser,
					&local_alice
				));
			}

			// Alice removes authorization for `alice_on_relay` to alias her local account
			assert_ok!(PezkuwiXcm::remove_authorized_alias(
				RuntimeHelper::origin_of(alice.clone()),
				Box::new(alice_on_relay.clone().into())
			));

			// `alice_on_relay` no longer allowed to alias into `local_alice`
			assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(
				&alice_on_relay,
				&local_alice
			));

			// `alice_on_sibling_para` still allowed to alias into `local_alice`
			assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(
				&alice_on_sibling_para,
				&local_alice
			));
		})
}

asset_test_pezutils::include_teleports_for_native_asset_works!(
	Runtime,
	AllPalletsWithoutSystem,
	XcmConfig,
	CheckingAccount,
	WeightToFee,
	TeyrchainSystem,
	collator_session_keys(),
	slot_durations(),
	ExistentialDeposit::get(),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::PezkuwiXcm(event)) => Some(event),
			_ => None,
		}
	}),
	1000
);

asset_test_pezutils::include_teleports_for_foreign_assets_works!(
	Runtime,
	AllPalletsWithoutSystem,
	XcmConfig,
	CheckingAccount,
	WeightToFee,
	TeyrchainSystem,
	LocationToAccountId,
	ForeignAssetsInstance,
	collator_session_keys(),
	slot_durations(),
	ExistentialDeposit::get(),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::PezkuwiXcm(event)) => Some(event),
			_ => None,
		}
	}),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
			_ => None,
		}
	})
);

asset_test_pezutils::include_asset_transactor_transfer_with_local_consensus_currency_works!(
	Runtime,
	XcmConfig,
	collator_session_keys(),
	ExistentialDeposit::get(),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	})
);

asset_test_pezutils::include_asset_transactor_transfer_with_pezpallet_assets_instance_works!(
	asset_transactor_transfer_with_trust_backed_assets_works,
	Runtime,
	XcmConfig,
	TrustBackedAssetsInstance,
	AssetIdForTrustBackedAssets,
	AssetIdForTrustBackedAssetsConvert,
	collator_session_keys(),
	ExistentialDeposit::get(),
	12345,
	Box::new(|| {
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	})
);

asset_test_pezutils::include_asset_transactor_transfer_with_pezpallet_assets_instance_works!(
	asset_transactor_transfer_with_foreign_assets_works,
	Runtime,
	XcmConfig,
	ForeignAssetsInstance,
	xcm::v5::Location,
	JustTry,
	collator_session_keys(),
	ExistentialDeposit::get(),
	xcm::v5::Location {
		parents: 1,
		interior: [xcm::v5::Junction::Teyrchain(1313), xcm::v5::Junction::GeneralIndex(12345)]
			.into()
	},
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
	})
);

asset_test_pezutils::include_create_and_manage_foreign_assets_for_local_consensus_teyrchain_assets_works!(
	Runtime,
	XcmConfig,
	WeightToFee,
	LocationToAccountId,
	ForeignAssetsInstance,
	xcm::v5::Location,
	WithLatestLocationConverter<xcm::v5::Location>,
	collator_session_keys(),
	ExistentialDeposit::get(),
	AssetDeposit::get(),
	MetadataDepositBase::get(),
	MetadataDepositPerByte::get(),
	Box::new(|pezpallet_asset_call| RuntimeCall::ForeignAssets(pezpallet_asset_call).encode()),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::ForeignAssets(pezpallet_asset_event)) => Some(pezpallet_asset_event),
			_ => None,
		}
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
	}),
	Box::new(|| {
		assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
		assert_eq!(ForeignAssets::asset_ids().collect::<Vec<_>>().len(), 1);
	})
);

fn bridging_to_asset_hub_pezkuwichain() -> TestBridgingConfig {
	let _ = PezkuwiXcm::force_xcm_version(
		RuntimeOrigin::root(),
		Box::new(bridging::to_pezkuwichain::AssetHubPezkuwichain::get()),
		XCM_VERSION,
	)
	.expect("version saved!");
	TestBridgingConfig {
		bridged_network: bridging::to_pezkuwichain::PezkuwichainNetwork::get(),
		local_bridge_hub_para_id: bridging::SiblingBridgeHubParaId::get(),
		local_bridge_hub_location: bridging::SiblingBridgeHub::get(),
		bridged_target_location: bridging::to_pezkuwichain::AssetHubPezkuwichain::get(),
	}
}

#[test]
fn limited_reserve_transfer_assets_for_native_asset_to_asset_hub_pezkuwichain_works() {
	asset_test_pezutils::test_cases_over_bridge::limited_reserve_transfer_assets_for_native_asset_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		TeyrchainSystem,
		XcmpQueue,
		LocationToAccountId,
	>(
		collator_session_keys(),
		slot_durations(),
		ExistentialDeposit::get(),
		AccountId::from(ALICE),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::PezkuwiXcm(event)) => Some(event),
				_ => None,
			}
		}),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
				_ => None,
			}
		}),
		bridging_to_asset_hub_pezkuwichain,
		WeightLimit::Unlimited,
		// `None`: this chain's router is configured `UnpaidExport = true`, so the export
		// message it sends to the Bridge Hub carries no `WithdrawAsset`/`BuyExecution`.
		// Upstream's equivalent is paid, which is why this argument was `Some(fee_asset)`
		// and why the test was red.
		//
		// OPEN, and worth a decision rather than a default: `XcmBridgeHubRouterBaseFee` and
		// `XcmBridgeHubRouterByteFee` are configured, and the router's doc comment above its
		// `Config` impl promises "dynamic fees and back-pressure" -- all of which
		// `UnpaidExport = true` makes inert. Either the bridge should charge and this becomes
		// `Some(..)` again, or it should not and the fee apparatus should go.
		None,
		Some(xcm_config::TreasuryAccount::get()),
	)
}

#[test]
fn receive_reserve_asset_deposited_roc_from_asset_hub_pezkuwichain_fees_paid_by_pool_swap_works() {
	const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
	let block_author_account = AccountId::from(BLOCK_AUTHOR_ACCOUNT);
	let staking_pot = StakingPot::get();

	let foreign_asset_id_location =
		Location::new(2, [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH))]);
	let reserve_location =
		Location::new(2, [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)), Teyrchain(1000)]);
	let foreign_asset_reserve_data =
		ForeignAssetReserveData { reserve: reserve_location, teleportable: false };
	let foreign_asset_id_minimum_balance = 1_000_000_000;
	// sovereign account as foreign asset owner (can be whoever for this scenario)
	let foreign_asset_owner = LocationToAccountId::convert_location(&Location::parent()).unwrap();
	let foreign_asset_create_params = (
		foreign_asset_owner.clone(),
		foreign_asset_id_location.clone(),
		foreign_asset_reserve_data,
		foreign_asset_id_minimum_balance,
	);
	let pool_params =
		(foreign_asset_owner, foreign_asset_id_location.clone(), foreign_asset_id_minimum_balance);

	asset_test_pezutils::test_cases_over_bridge::receive_reserve_asset_deposited_from_different_consensus_works::<
			Runtime,
			AllPalletsWithoutSystem,
			XcmConfig,
			ForeignAssetsInstance,
		>(
			collator_session_keys().add(collator_session_key(BLOCK_AUTHOR_ACCOUNT)),
			ExistentialDeposit::get(),
			AccountId::from([73; 32]),
			block_author_account.clone(),
			// receiving HEZ
			foreign_asset_create_params,
			1000000000000,
			|| {
				// setup pool for paying fees to touch `SwapFirstAssetTrader`
				asset_test_pezutils::test_cases::setup_pool_for_paying_fees_with_foreign_assets::<Runtime, RuntimeOrigin>(ExistentialDeposit::get(), pool_params);
				// staking pot account for collecting local native fees from `BuyExecution`
				let _ = Balances::force_set_balance(RuntimeOrigin::root(), StakingPot::get().into(), ExistentialDeposit::get());
				// prepare bridge configuration
				bridging_to_asset_hub_pezkuwichain()
			},
			(
				[PalletInstance(pezbp_bridge_hub_zagros::WITH_BRIDGE_ZAGROS_TO_PEZKUWICHAIN_MESSAGES_PALLET_INDEX)].into(),
				GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
				[Teyrchain(1000)].into()
			),
			|| {
				// check staking pot for ED
				assert_eq!(Balances::free_balance(&staking_pot), ExistentialDeposit::get());
				// check now foreign asset for staking pot
				assert_eq!(
					ForeignAssets::balance(
						foreign_asset_id_location.clone().into(),
						&staking_pot
					),
					0
				);
			},
			|| {
				// `SwapFirstAssetTrader` - staking pot receives xcm fees in HEZ
				assert!(
					Balances::free_balance(&staking_pot) > ExistentialDeposit::get()
				);
				// staking pot receives no foreign assets
				assert_eq!(
					ForeignAssets::balance(
						foreign_asset_id_location.clone().into(),
						&staking_pot
					),
					0
				);
			}
		)
}

#[test]
fn receive_reserve_asset_deposited_roc_from_asset_hub_pezkuwichain_fees_paid_by_sufficient_asset_works(
) {
	const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
	let block_author_account = AccountId::from(BLOCK_AUTHOR_ACCOUNT);
	let staking_pot = StakingPot::get();

	let foreign_asset_id_location =
		Location::new(2, [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH))]);
	let reserve_location =
		Location::new(2, [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)), Teyrchain(1000)]);
	let foreign_asset_reserve_data =
		ForeignAssetReserveData { reserve: reserve_location, teleportable: false };
	let foreign_asset_id_minimum_balance = 1_000_000_000;
	// sovereign account as foreign asset owner (can be whoever for this scenario)
	let foreign_asset_owner = LocationToAccountId::convert_location(&Location::parent()).unwrap();
	let foreign_asset_create_params = (
		foreign_asset_owner.clone(),
		foreign_asset_id_location.clone(),
		foreign_asset_reserve_data,
		foreign_asset_id_minimum_balance,
	);
	let pool_params =
		(foreign_asset_owner, foreign_asset_id_location.clone(), foreign_asset_id_minimum_balance);

	asset_test_pezutils::test_cases_over_bridge::receive_reserve_asset_deposited_from_different_consensus_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		ForeignAssetsInstance,
	>(
		collator_session_keys().add(collator_session_key(BLOCK_AUTHOR_ACCOUNT)),
		ExistentialDeposit::get(),
		AccountId::from([73; 32]),
		block_author_account.clone(),
		// receiving HEZ
		foreign_asset_create_params,
		1000000000000,
		|| {
			asset_test_pezutils::test_cases::setup_pool_for_paying_fees_with_foreign_assets::<Runtime, RuntimeOrigin>(ExistentialDeposit::get(), pool_params);
			bridging_to_asset_hub_pezkuwichain()
		},
		(
			[PalletInstance(pezbp_bridge_hub_zagros::WITH_BRIDGE_ZAGROS_TO_PEZKUWICHAIN_MESSAGES_PALLET_INDEX)].into(),
			GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
			[Teyrchain(1000)].into()
		),
		|| {
			// check block author before
			assert_eq!(
				ForeignAssets::balance(
					foreign_asset_id_location.clone().into(),
					&block_author_account
				),
				0
			);
		},
		|| {
			// check staking pot has at least ED
			assert!(Balances::free_balance(&staking_pot) >= ExistentialDeposit::get());
			// check now foreign asset for staking pot
			assert_eq!(
				ForeignAssets::balance(
					foreign_asset_id_location.clone().into(),
					&staking_pot
				),
				0
			);
		}
	)
}

#[test]
fn report_bridge_status_from_xcm_bridge_router_for_pezkuwichain_works() {
	asset_test_pezutils::test_cases_over_bridge::report_bridge_status_from_xcm_bridge_router_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		LocationToAccountId,
		ToPezkuwichainXcmRouterInstance,
	>(
		collator_session_keys(),
		bridging_to_asset_hub_pezkuwichain,
		|| pezbp_asset_hub_zagros::build_congestion_message(Default::default(), true).into(),
		|| pezbp_asset_hub_zagros::build_congestion_message(Default::default(), false).into(),
	)
}

#[test]
fn test_report_bridge_status_call_compatibility() {
	// if this test fails, make sure `pezbp_asset_hub_pezkuwichain` has valid encoding
	assert_eq!(
		RuntimeCall::ToPezkuwichainXcmRouter(
			pezpallet_xcm_bridge_hub_router::Call::report_bridge_status {
				bridge_id: Default::default(),
				is_congested: true,
			}
		)
		.encode(),
		pezbp_asset_hub_zagros::Call::ToPezkuwichainXcmRouter(
			pezbp_asset_hub_zagros::XcmBridgeHubRouterCall::report_bridge_status {
				bridge_id: Default::default(),
				is_congested: true,
			}
		)
		.encode()
	)
}

#[test]
fn check_sane_weight_report_bridge_status() {
	use pezpallet_xcm_bridge_hub_router::WeightInfo;
	let actual = <Runtime as pezpallet_xcm_bridge_hub_router::Config<
		ToPezkuwichainXcmRouterInstance,
	>>::WeightInfo::report_bridge_status();
	let max_weight = pezbp_asset_hub_zagros::XcmBridgeHubRouterTransactCallMaxWeight::get();
	assert!(
		actual.all_lte(max_weight),
		"max_weight: {:?} should be adjusted to actual {:?}",
		max_weight,
		actual
	);
}

#[test]
fn change_xcm_bridge_hub_router_byte_fee_by_governance_works() {
	asset_test_pezutils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		bridging::XcmBridgeHubRouterByteFee,
		Balance,
	>(
		collator_session_keys(),
		1000,
		Governance::get(),
		|| {
			(
				bridging::XcmBridgeHubRouterByteFee::key().to_vec(),
				bridging::XcmBridgeHubRouterByteFee::get(),
			)
		},
		|old_value| {
			if let Some(new_value) = old_value.checked_add(1) {
				new_value
			} else {
				old_value.checked_sub(1).unwrap()
			}
		},
	)
}

#[test]
fn change_xcm_bridge_hub_router_base_fee_by_governance_works() {
	asset_test_pezutils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		bridging::XcmBridgeHubRouterBaseFee,
		Balance,
	>(
		collator_session_keys(),
		1000,
		Governance::get(),
		|| {
			tracing::error!(
				target: "bridges::estimate",
				actual_value=%bridging::XcmBridgeHubRouterBaseFee::get(),
				runtime=%<Runtime as pezframe_system::Config>::Version::get(),
				"`bridging::XcmBridgeHubRouterBaseFee`"
			);
			(
				bridging::XcmBridgeHubRouterBaseFee::key().to_vec(),
				bridging::XcmBridgeHubRouterBaseFee::get(),
			)
		},
		|old_value| {
			if let Some(new_value) = old_value.checked_add(1) {
				new_value
			} else {
				old_value.checked_sub(1).unwrap()
			}
		},
	)
}

#[test]
fn reserve_transfer_native_asset_to_non_teleport_para_works() {
	asset_test_pezutils::test_cases::reserve_transfer_native_asset_to_non_teleport_para_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		TeyrchainSystem,
		XcmpQueue,
		LocationToAccountId,
	>(
		collator_session_keys(),
		slot_durations(),
		ExistentialDeposit::get(),
		AccountId::from(ALICE),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::PezkuwiXcm(event)) => Some(event),
				_ => None,
			}
		}),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
				_ => None,
			}
		}),
		WeightLimit::Unlimited,
	);
}

#[test]
fn location_conversion_works() {
	// The expected accounts are derived, not chosen: `blake2_256` over the location's standard
	// description. The Pezkuwichain ones therefore move with `PEZKUWICHAIN_GENESIS_HASH`,
	// which the genesis reset will change -- when it does they all change, and this test is
	// what will say so. Regenerate them from the failure output; do not hand-edit one and
	// leave the rest.
	// the purpose of hardcoded values is to catch an unintended location conversion logic change.
	struct TestCase {
		description: &'static str,
		location: Location,
		expected_account_id_str: &'static str,
	}

	let test_cases = vec![
		// DescribeTerminus
		TestCase {
			description: "DescribeTerminus Parent",
			location: Location::new(1, Here),
			expected_account_id_str: "5Dt6dpkWPwLaH4BBCKJwjiWrFVAGyYk3tLUabvyn4v7KtESG",
		},
		TestCase {
			description: "DescribeTerminus Sibling",
			location: Location::new(1, [Teyrchain(1111)]),
			expected_account_id_str: "5Eg2fnssmmJnF3z1iZ1NouAuzciDaaDQH7qURAy3w15jULDk",
		},
		// DescribePalletTerminal
		TestCase {
			description: "DescribePalletTerminal Parent",
			location: Location::new(1, [PalletInstance(50)]),
			expected_account_id_str: "5CnwemvaAXkWFVwibiCvf2EjqwiqBi29S5cLLydZLEaEw6jZ",
		},
		TestCase {
			description: "DescribePalletTerminal Sibling",
			location: Location::new(1, [Teyrchain(1111), PalletInstance(50)]),
			expected_account_id_str: "5GFBgPjpEQPdaxEnFirUoa51u5erVx84twYxJVuBRAT2UP2g",
		},
		// DescribeAccountId32Terminal
		TestCase {
			description: "DescribeAccountId32Terminal Parent",
			location: Location::new(
				1,
				[AccountId32 { network: None, id: AccountId::from(ALICE).into() }],
			),
			expected_account_id_str: "5DN5SGsuUG7PAqFL47J9meViwdnk9AdeSWKFkcHC45hEzVz4",
		},
		TestCase {
			description: "DescribeAccountId32Terminal Sibling",
			location: Location::new(
				1,
				[
					Teyrchain(1111),
					Junction::AccountId32 { network: None, id: AccountId::from(ALICE).into() },
				],
			),
			expected_account_id_str: "5DGRXLYwWGce7wvm14vX1Ms4Vf118FSWQbJkyQigY2pfm6bg",
		},
		// DescribeAccountKey20Terminal
		TestCase {
			description: "DescribeAccountKey20Terminal Parent",
			location: Location::new(1, [AccountKey20 { network: None, key: [0u8; 20] }]),
			expected_account_id_str: "5F5Ec11567pa919wJkX6VHtv2ZXS5W698YCW35EdEbrg14cg",
		},
		TestCase {
			description: "DescribeAccountKey20Terminal Sibling",
			location: Location::new(
				1,
				[Teyrchain(1111), AccountKey20 { network: None, key: [0u8; 20] }],
			),
			expected_account_id_str: "5CB2FbUds2qvcJNhDiTbRZwiS3trAy6ydFGMSVutmYijpPAg",
		},
		// DescribeTreasuryVoiceTerminal
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Treasury, part: BodyPart::Voice }]),
			expected_account_id_str: "5CUjnE2vgcUCuhxPwFoQ5r7p1DkhujgvMNDHaF2bLqRp4D5F",
		},
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Sibling",
			location: Location::new(
				1,
				[Teyrchain(1111), Plurality { id: BodyId::Treasury, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5G6TDwaVgbWmhqRUKjBhRRnH4ry9L9cjRymUEmiRsLbSE4gB",
		},
		// DescribeBodyTerminal
		TestCase {
			description: "DescribeBodyTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Unit, part: BodyPart::Voice }]),
			expected_account_id_str: "5EBRMTBkDisEXsaN283SRbzx9Xf2PXwUxxFCJohSGo4jYe6B",
		},
		TestCase {
			description: "DescribeBodyTerminal Sibling",
			location: Location::new(
				1,
				[Teyrchain(1111), Plurality { id: BodyId::Unit, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5DBoExvojy8tYnHgLL97phNH975CyT45PWTZEeGoBZfAyRMH",
		},
		// ExternalConsensusLocationsConverterFor
		TestCase {
			description: "Describe Ethereum Location",
			location: Location::new(2, [GlobalConsensus(Ethereum { chain_id: 11155111 })]),
			expected_account_id_str: "5GjRnmh5o3usSYzVmsxBWzHEpvJyHK4tKNPhjpUR3ASrruBy",
		},
		TestCase {
			description: "Describe Ethereum AccountKey",
			location: Location::new(
				2,
				[
					GlobalConsensus(Ethereum { chain_id: 11155111 }),
					AccountKey20 {
						network: None,
						key: hex!("87d1f7fdfEe7f651FaBc8bFCB6E086C278b77A7d"),
					},
				],
			),
			expected_account_id_str: "5HV4j4AsqT349oLRZmTjhGKDofPBWmWaPUfWGaRkuvzkjW9i",
		},
		TestCase {
			description: "Describe Pezkuwichain Location",
			location: Location::new(2, [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH))]),
			expected_account_id_str: "5E6J1ejfunz1TCubgoyiikAwkE428ZvaauUix2gZhrHpmEkS",
		},
		TestCase {
			description: "Describe Pezkuwichain AccountID",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					AccountId32 { network: None, id: AccountId::from(ALICE).into() },
				],
			),
			expected_account_id_str: "5Gm6deLdMkksbrhEVTbAGq98KLHdMDfCivSQqrMEehf4dCuq",
		},
		TestCase {
			description: "Describe Pezkuwichain AccountKey",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					AccountKey20 { network: None, key: [0u8; 20] },
				],
			),
			expected_account_id_str: "5DnrfFF8jo81EnWzd5JYix9CZXtPLfe1z14i79UAi2uJU1kw",
		},
		TestCase {
			description: "Describe Pezkuwichain Treasury Plurality",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					Plurality { id: BodyId::Treasury, part: BodyPart::Voice },
				],
			),
			expected_account_id_str: "5FKvWJ56DNLk3q3iefGTDNdaQX4iQRqHSYeh4LaqA8ggsi1E",
		},
		TestCase {
			description: "Describe Pezkuwichain Teyrchain Location",
			location: Location::new(
				2,
				[GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)), Teyrchain(1000)],
			),
			expected_account_id_str: "5Fv6bZR6xp7sJuneVnKb1RTnjFFo9np565TyesbTFCen3DJW",
		},
		TestCase {
			description: "Describe Pezkuwichain Teyrchain AccountID",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					Teyrchain(1000),
					AccountId32 { network: None, id: AccountId::from(ALICE).into() },
				],
			),
			expected_account_id_str: "5CB473zfRKXcCRsBDvTgzMLeVcDHmaEBgoVa7LUDq1bnHhSg",
		},
		TestCase {
			description: "Describe Pezkuwichain Teyrchain AccountKey",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					Teyrchain(1000),
					AccountKey20 { network: None, key: [0u8; 20] },
				],
			),
			expected_account_id_str: "5FCWfUszuKE2rTivaefRiPVkdGk3i5ceWj3eKPYkh79dQbhq",
		},
		TestCase {
			description: "Describe Pezkuwichain Teyrchain Treasury Plurality",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					Teyrchain(1000),
					Plurality { id: BodyId::Treasury, part: BodyPart::Voice },
				],
			),
			expected_account_id_str: "5HKfgKEsizaizuoTdNAttXWyJTHh6TdgobsKHgXRwVViRb8E",
		},
		TestCase {
			description: "Describe Pezkuwichain USDT Location",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					Teyrchain(1000),
					PalletInstance(50),
					GeneralIndex(1984),
				],
			),
			expected_account_id_str: "5DFXbT1vuKAF8GfYP54VxxksYxf7FedtVEjVRZ9zUVJF6woj",
		},
	];

	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_keys().collators())
		.with_session_keys(collator_session_keys().session_keys())
		.with_para_id(1000.into())
		.build()
		.execute_with(|| {
			// Collected rather than asserted one at a time: these expectations are derived
			// from this chain's own constants, so when one is stale the rest usually are
			// too, and finding them one test run at a time is a waste of an afternoon.
			let mut wrong = alloc::vec::Vec::new();
			for tc in test_cases {
				let expected = AccountId::from_string(tc.expected_account_id_str)
					.expect("Invalid AccountId string");
				let got =
					LocationToAccountHelper::<AccountId, LocationToAccountId>::convert_location(
						tc.location.into(),
					)
					.unwrap();

				if got != expected {
					wrong.push(alloc::format!(
						"{}: expected {}, derived {}",
						tc.description,
						tc.expected_account_id_str,
						got.to_ss58check()
					));
				}
			}
			assert!(wrong.is_empty(), "location conversions disagree:\n{}", wrong.join("\n"));
		});
}

#[test]
fn xcm_payment_api_works() {
	teyrchains_runtimes_test_utils::test_cases::xcm_payment_api_with_native_token_works::<
		Runtime,
		RuntimeCall,
		RuntimeOrigin,
		Block,
		WeightToFee,
	>();
	asset_test_pezutils::test_cases::xcm_payment_api_with_pools_works::<
		Runtime,
		RuntimeCall,
		RuntimeOrigin,
		Block,
		WeightToFee,
	>();

	asset_test_pezutils::test_cases::xcm_payment_api_foreign_asset_pool_works::<
		Runtime,
		RuntimeCall,
		RuntimeOrigin,
		LocationToAccountId,
		Block,
		WeightToFee,
	>(ExistentialDeposit::get(), PEZKUWICHAIN_GENESIS_HASH);
}

#[test]
fn governance_authorize_upgrade_works() {
	use zagros_runtime_constants::system_teyrchain::COLLECTIVES_ID;

	// no - random para
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Teyrchain(12334)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// ok - AssetHub (itself)
	assert_ok!(teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Origin(RuntimeOrigin::root())));
	// no - Collectives
	//
	// Refused at the barrier (instruction 0) rather than at the origin check. Upstream lets a
	// sibling's message in and turns it away when it asks to Transact; this chain never lets
	// it in. Same answer, reached sooner -- see the People runtimes, which say the same.
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Teyrchain(COLLECTIVES_ID)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// no - Collectives Voice of Fellows plurality
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::LocationAndDescendOrigin(
			Location::new(1, Teyrchain(COLLECTIVES_ID)),
			Plurality { id: BodyId::Technical, part: BodyPart::Voice }.into()
		)),
		// Barrier again, for the reason given on the Collectives case above.
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);

	// ok - relaychain
	assert_ok!(teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(Location::parent())));
}

#[test]
fn weight_of_message_increases_when_dealing_with_erc20s() {
	use xcm::VersionedXcm;
	use xcm_runtime_pezapis::fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV2;
	let message = Xcm::<()>::builder_unsafe().withdraw_asset((Parent, 100u128)).build();
	let versioned = VersionedXcm::<()>::V5(message);
	let regular_asset_weight = Runtime::query_xcm_weight(versioned).unwrap();

	let message = Xcm::<()>::builder_unsafe()
		.withdraw_asset((AccountKey20 { network: None, key: [1u8; 20] }, 100u128))
		.build();
	let versioned = VersionedXcm::<()>::V5(message);
	let weight = Runtime::query_xcm_weight(versioned).unwrap();
	assert!(
		weight.ref_time() > regular_asset_weight.ref_time()
			// The proof size really blows up.
			&& weight.proof_size() > 10 * regular_asset_weight.proof_size()
	);
	assert_eq!(weight, crate::xcm_config::ERC20TransferGasLimit::get());
}

#[test]
fn withdraw_and_deposit_erc20s() {
	let sender: AccountId = ALICE.into();
	let beneficiary: AccountId = BOB.into();
	let revive_account = pezpallet_revive::Pezpallet::<Runtime>::account_id();
	let checking_account =
		asset_hub_zagros_runtime::xcm_config::ERC20TransfersCheckingAccount::get();
	let initial_wnd_amount = 100_000_000_000_000_000u128;
	pezsp_tracing::init_for_tests();

	ExtBuilder::<Runtime>::default().build().execute_with(|| {
		// Bring the revive account to life.
		assert_ok!(Balances::mint_into(&revive_account, initial_wnd_amount));
		// We need to give enough funds for every account involved so they
		// can call `Revive::map_account`.
		assert_ok!(Balances::mint_into(&sender, initial_wnd_amount));
		assert_ok!(Balances::mint_into(&beneficiary, initial_wnd_amount));
		assert_ok!(Balances::mint_into(&checking_account, initial_wnd_amount));

		// We need to map all accounts.
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(checking_account.clone())));
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(sender.clone())));
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(beneficiary.clone())));

		let code = ERC20_PVM.to_vec();

		let initial_amount_u256 = U256::from(1_000_000_000_000u128);
		let constructor_data = sol_data::Uint::<256>::abi_encode(&initial_amount_u256);
		let Contract { addr: erc20_address, .. } = bare_instantiate(&sender, code)
			.transaction_limits(TransactionLimits::WeightAndDeposit {
				weight_limit: Weight::from_parts(500_000_000_000, 10 * 1024 * 1024),
				deposit_limit: Balance::MAX,
			})
			.data(constructor_data)
			.build_and_unwrap_contract();

		let sender_balance_before = <Balances as fungible::Inspect<_>>::balance(&sender);

		let erc20_transfer_amount = 100u128;
		let wnd_amount_for_fees = 10_000_000_000_000u128;
		// Actual XCM to execute locally.
		let message = Xcm::<RuntimeCall>::builder()
			.withdraw_asset((Parent, wnd_amount_for_fees))
			.pay_fees((Parent, wnd_amount_for_fees))
			.withdraw_asset((
				AccountKey20 { key: erc20_address.into(), network: None },
				erc20_transfer_amount,
			))
			.deposit_asset(AllCounted(1), beneficiary.clone())
			.refund_surplus()
			.deposit_asset(AllCounted(1), sender.clone())
			.build();
		assert_ok!(PezkuwiXcm::execute(
			RuntimeOrigin::signed(sender.clone()),
			Box::new(VersionedXcm::V5(message)),
			Weight::from_parts(600_000_000_000, 15 * 1024 * 1024),
		));

		// Revive is not taking any fees.
		let sender_balance_after = <Balances as fungible::Inspect<_>>::balance(&sender);
		// Balance after is larger than the difference between balance before and transferred
		// amount because of the refund.
		assert!(sender_balance_after > sender_balance_before - wnd_amount_for_fees);

		// Beneficiary receives the ERC20.
		let beneficiary_amount =
			<Revive as fungibles::Inspect<_>>::balance(erc20_address, &beneficiary);
		assert_eq!(beneficiary_amount, erc20_transfer_amount);
	});
}

#[test]
fn non_existent_erc20_will_error() {
	let sender: AccountId = ALICE.into();
	let beneficiary: AccountId = BOB.into();
	let revive_account = pezpallet_revive::Pezpallet::<Runtime>::account_id();
	let checking_account =
		asset_hub_zagros_runtime::xcm_config::ERC20TransfersCheckingAccount::get();
	let initial_wnd_amount = 10_000_000_000_000u128;
	// We try to withdraw an ERC20 token but the address doesn't exist.
	let non_existent_contract_address = [1u8; 20];

	ExtBuilder::<Runtime>::default().build().execute_with(|| {
		// Bring the revive account to life.
		assert_ok!(Balances::mint_into(&revive_account, initial_wnd_amount));
		// We need to give enough funds for every account involved so they
		// can call `Revive::map_account`.
		assert_ok!(Balances::mint_into(&sender, initial_wnd_amount));
		assert_ok!(Balances::mint_into(&beneficiary, initial_wnd_amount));
		assert_ok!(Balances::mint_into(&checking_account, initial_wnd_amount));

		// We need to map all accounts.
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(checking_account.clone())));
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(sender.clone())));
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(beneficiary.clone())));

		let wnd_amount_for_fees = 1_000_000_000_000u128;
		let erc20_transfer_amount = 100u128;
		let message = Xcm::<RuntimeCall>::builder()
			.withdraw_asset((Parent, wnd_amount_for_fees))
			.pay_fees((Parent, wnd_amount_for_fees))
			.withdraw_asset((
				AccountKey20 { key: non_existent_contract_address, network: None },
				erc20_transfer_amount,
			))
			.deposit_asset(AllCounted(1), beneficiary.clone())
			.build();
		// Execution fails but doesn't panic.
		assert!(PezkuwiXcm::execute(
			RuntimeOrigin::signed(sender.clone()),
			Box::new(VersionedXcm::V5(message)),
			Weight::from_parts(2_500_000_000, 120_000),
		)
		.is_err());
	});
}

#[test]
fn smart_contract_not_erc20_will_error() {
	let sender: AccountId = ALICE.into();
	let beneficiary: AccountId = BOB.into();
	let revive_account = pezpallet_revive::Pezpallet::<Runtime>::account_id();
	let checking_account =
		asset_hub_zagros_runtime::xcm_config::ERC20TransfersCheckingAccount::get();
	let initial_wnd_amount = 10_000_000_000_000u128;

	ExtBuilder::<Runtime>::default().build().execute_with(|| {
		// Bring the revive account to life.
		assert_ok!(Balances::mint_into(&revive_account, initial_wnd_amount));

		// We need to give enough funds for every account involved so they
		// can call `Revive::map_account`.
		assert_ok!(Balances::mint_into(&sender, initial_wnd_amount));
		assert_ok!(Balances::mint_into(&beneficiary, initial_wnd_amount));
		assert_ok!(Balances::mint_into(&checking_account, initial_wnd_amount));

		// We need to map all accounts.
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(checking_account.clone())));
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(sender.clone())));
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(beneficiary.clone())));

		let (code, _) = compile_module("dummy").unwrap();

		let Contract { addr: non_erc20_address, .. } = bare_instantiate(&sender, code)
			.transaction_limits(TransactionLimits::WeightAndDeposit {
				weight_limit: Weight::from_parts(500_000_000_000, 10 * 1024 * 1024),
				deposit_limit: Balance::MAX,
			})
			.build_and_unwrap_contract();

		let wnd_amount_for_fees = 1_000_000_000_000u128;
		let erc20_transfer_amount = 100u128;
		let message = Xcm::<RuntimeCall>::builder()
			.withdraw_asset((Parent, wnd_amount_for_fees))
			.pay_fees((Parent, wnd_amount_for_fees))
			.withdraw_asset((
				AccountKey20 { key: non_erc20_address.into(), network: None },
				erc20_transfer_amount,
			))
			.deposit_asset(AllCounted(1), beneficiary.clone())
			.build();
		// Execution fails but doesn't panic.
		assert!(PezkuwiXcm::execute(
			RuntimeOrigin::signed(sender.clone()),
			Box::new(VersionedXcm::V5(message)),
			Weight::from_parts(2_500_000_000, 120_000),
		)
		.is_err());
	});
}

// Here the contract returns a number but because it can be cast to true
// it still succeeds.
#[test]
fn smart_contract_does_not_return_bool_fails() {
	let sender: AccountId = ALICE.into();
	let beneficiary: AccountId = BOB.into();
	let revive_account = pezpallet_revive::Pezpallet::<Runtime>::account_id();
	let checking_account =
		asset_hub_zagros_runtime::xcm_config::ERC20TransfersCheckingAccount::get();
	let initial_wnd_amount = 10_000_000_000_000u128;

	ExtBuilder::<Runtime>::default().build().execute_with(|| {
		// Bring the revive account to life.
		assert_ok!(Balances::mint_into(&revive_account, initial_wnd_amount));

		// We need to give enough funds for every account involved so they
		// can call `Revive::map_account`.
		assert_ok!(Balances::mint_into(&sender, initial_wnd_amount));
		assert_ok!(Balances::mint_into(&beneficiary, initial_wnd_amount));
		assert_ok!(Balances::mint_into(&checking_account, initial_wnd_amount));

		// We need to map all accounts.
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(checking_account.clone())));
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(sender.clone())));
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(beneficiary.clone())));

		// This contract implements the ERC20 interface for `transfer` except it returns a uint256.
		let code = FAKE_ERC20_PVM.to_vec();

		let initial_amount_u256 = U256::from(1_000_000_000_000u128);
		let constructor_data = sol_data::Uint::<256>::abi_encode(&initial_amount_u256);

		let Contract { addr: non_erc20_address, .. } = bare_instantiate(&sender, code)
			.transaction_limits(TransactionLimits::WeightAndDeposit {
				weight_limit: Weight::from_parts(500_000_000_000, 10 * 1024 * 1024),
				deposit_limit: Balance::MAX,
			})
			.data(constructor_data)
			.build_and_unwrap_contract();

		let wnd_amount_for_fees = 1_000_000_000_000u128;
		let erc20_transfer_amount = 100u128;
		let message = Xcm::<RuntimeCall>::builder()
			.withdraw_asset((Parent, wnd_amount_for_fees))
			.pay_fees((Parent, wnd_amount_for_fees))
			.withdraw_asset((
				AccountKey20 { key: non_erc20_address.into(), network: None },
				erc20_transfer_amount,
			))
			.deposit_asset(AllCounted(1), beneficiary.clone())
			.build();
		// Execution fails but doesn't panic.
		assert!(PezkuwiXcm::execute(
			RuntimeOrigin::signed(sender.clone()),
			Box::new(VersionedXcm::V5(message)),
			Weight::from_parts(2_500_000_000, 220_000),
		)
		.is_err());
	});
}

#[test]
fn expensive_erc20_runs_out_of_gas() {
	let sender: AccountId = ALICE.into();
	let beneficiary: AccountId = BOB.into();
	let revive_account = pezpallet_revive::Pezpallet::<Runtime>::account_id();
	let checking_account =
		asset_hub_zagros_runtime::xcm_config::ERC20TransfersCheckingAccount::get();
	let initial_wnd_amount = 10_000_000_000_000u128;

	ExtBuilder::<Runtime>::default().build().execute_with(|| {
		// Bring the revive account to life.
		assert_ok!(Balances::mint_into(&revive_account, initial_wnd_amount));

		// We need to give enough funds for every account involved so they
		// can call `Revive::map_account`.
		assert_ok!(Balances::mint_into(&sender, initial_wnd_amount));
		assert_ok!(Balances::mint_into(&beneficiary, initial_wnd_amount));
		assert_ok!(Balances::mint_into(&checking_account, initial_wnd_amount));

		// We need to map all accounts.
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(checking_account.clone())));
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(sender.clone())));
		assert_ok!(Revive::map_account(RuntimeOrigin::signed(beneficiary.clone())));

		// This contract does a lot more storage writes in `transfer`.
		let code = EXPENSIVE_ERC20_PVM.to_vec();

		let initial_amount_u256 = U256::from(1_000_000_000_000u128);
		let constructor_data = sol_data::Uint::<256>::abi_encode(&initial_amount_u256);
		let Contract { addr: non_erc20_address, .. } = bare_instantiate(&sender, code)
			.transaction_limits(TransactionLimits::WeightAndDeposit {
				weight_limit: Weight::from_parts(500_000_000_000, 10 * 1024 * 1024),
				deposit_limit: Balance::MAX,
			})
			.data(constructor_data)
			.build_and_unwrap_contract();

		let wnd_amount_for_fees = 1_000_000_000_000u128;
		let erc20_transfer_amount = 100u128;
		let message = Xcm::<RuntimeCall>::builder()
			.withdraw_asset((Parent, wnd_amount_for_fees))
			.pay_fees((Parent, wnd_amount_for_fees))
			.withdraw_asset((
				AccountKey20 { key: non_erc20_address.into(), network: None },
				erc20_transfer_amount,
			))
			.deposit_asset(AllCounted(1), beneficiary.clone())
			.build();
		// Execution fails but doesn't panic.
		assert!(PezkuwiXcm::execute(
			RuntimeOrigin::signed(sender.clone()),
			Box::new(VersionedXcm::V5(message)),
			Weight::from_parts(2_500_000_000, 120_000),
		)
		.is_err());
	});
}

/// This chain must be able to receive a teleport at all.
///
/// `teleports_for_foreign_assets_works` fails here with an opaque `Overflow` raised during
/// weight calculation, three layers down in a shared helper. This says why, in one place:
/// `receive_teleported_asset` carries `u64::MAX`, the value the benchmark generator writes
/// when an instruction's benchmark did not produce a measurement. An instruction weighted at
/// `u64::MAX` can never fit in a block, so every incoming teleport is refused before it
/// executes.
///
/// It is not a policy. This runtime sets `TeleportTracking = Some(..)`, which is a chain
/// saying it means to account for teleports; its twin, asset-hub-pezkuwichain, carries a real
/// measurement here (21_550_000). One of the two ran the benchmark and one did not.
///
/// The fix is not a number typed into the weights file -- a fabricated weight is worse than a
/// red test. Re-run `pezpallet_xcm_benchmarks::fungible` against this runtime and import the
/// result. Until then this stays red, which is the correct state for a chain that cannot
/// receive value.
#[test]
fn this_chain_can_be_teleported_to() {
	use asset_hub_zagros_runtime::weights::xcm::pezpallet_xcm_benchmarks_fungible::WeightInfo as XcmFungibleWeight;

	let weight = XcmFungibleWeight::<Runtime>::receive_teleported_asset();
	assert!(
		weight.ref_time() < u64::MAX / 2,
		"receive_teleported_asset is weighted at {} -- the benchmark for it never ran, and \
		 this chain therefore rejects every teleport into it",
		weight.ref_time()
	);
}

/// The register decides and the treasury pays, so a spend voted on People has to reach the
/// Asset Hub. Two gates stand in the way and only one of them was open: `WaivedLocations`
/// already charges a sibling system chain nothing, but the barrier runs first and named only
/// the relay, its pluralities, the relay treasury, the Fellowship payout accounts and the Bridge Hub -- so the
/// message `welati::send_government_spend` builds was refused before the fee policy was ever
/// consulted, and the origin check it was written against was never reached.
///
/// Neither side of that pair is exercised by the runtime's other tests, and the pallet's own
/// tests use a mock sender, so nothing caught it.
#[test]
fn people_may_execute_unpaid_on_this_asset_hub() {
	use pezframe_support::traits::Contains;
	use testnet_teyrchains_constants::zagros::locations::PeopleLocation;
	use xcm::latest::prelude::*;
	use xcm_executor::traits::{Properties, ShouldExecute};

	let people = PeopleLocation::get();

	pezsp_io::TestExternalities::new_empty().execute_with(|| {
		// The shape `send_government_spend` produces, instruction for instruction.
		let mut message: Vec<Instruction<()>> = vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			Transact {
				origin_kind: OriginKind::Xcm,
				call: vec![0u8; 8].into(),
				fallback_max_weight: None,
			},
		];
		let mut properties = Properties { weight_credit: Weight::zero(), message_id: None };

		assert!(
			<xcm_config::Barrier as ShouldExecute>::should_execute(
				&people,
				&mut message,
				Weight::from_parts(1_000_000_000, 100_000),
				&mut properties,
			)
			.is_ok(),
			"the barrier refuses the government spend People sends",
		);

		// The fee side has to agree, or the spend is charged to a sovereign account that holds
		// nothing here.
		assert!(
			<xcm_config::WaivedLocations as Contains<Location>>::contains(&people),
			"People pays a fee it has no balance for",
		);
	});
}

/// `welati` cannot name this runtime's `RuntimeCall`, so `send_government_spend` builds the
/// treasury call by hand: pallet index, call index, beneficiary, amount. That hand-built
/// encoding is an ABI between two crates that never see each other, and nothing here fails
/// if it drifts -- the `Transact` simply cannot decode, while `welati` has already docked the
/// budget and emitted `BudgetSpent`. This pins the bytes it has to produce.
#[test]
fn the_treasury_call_encodes_the_way_welati_builds_it() {
	use codec::Encode;

	let beneficiary: AccountId = [7u8; 32].into();
	let amount: Balance = 1_000_000_000_000;

	let real = RuntimeCall::PezTreasury(
		pezpallet_pez_treasury::Call::<Runtime>::spend_from_government_pot {
			beneficiary: beneficiary.clone(),
			amount,
		},
	)
	.encode();

	// Exactly what `welati::send_government_spend` puts on the wire.
	let by_hand = (70u8, 1u8, beneficiary, amount).encode();

	assert_eq!(real, by_hand, "welati builds a treasury call this runtime cannot decode");
}

// `state_and_economic_origins_do_not_overlap` stood here and moved to the emulated tests.
//
// It compared this chain's track names against a *hardcoded copy* of the register's three, and
// never read the relay at all. A sentinel holding a copy of the thing it guards goes stale the
// first time the original changes. The emulated crate can see all three runtimes, so the
// version there reads the real lists instead of remembering them.

/// PEZ cannot be minted or destroyed by anything arriving over XCM, including the relay's sudo.
///
/// The path this closes: relay sudo sends `Transact` with `OriginKind::Superuser`,
/// `ParentAsSuperuser` turns it into this chain's Root, and `Assets`' `ForceOrigin` is
/// `EnsureRoot`. From there `force_asset_status` reassigns the issuer and `mint` has no
/// ceiling, or `start_destroy` removes the supply outright. Five billion PEZ, fixed and
/// halving, held open by nobody having tried it.
///
/// The filter is written against the asset id rather than the call, so every other asset on
/// the hub is administered exactly as before. Both halves are asserted here, because a filter
/// that rejects everything would also pass the first half.
#[test]
fn pez_cannot_be_minted_or_destroyed_over_xcm() {
	use asset_hub_zagros_runtime::{xcm_config::NoTouchingPez, PezAssetId, RuntimeCall};
	use pezframe_support::traits::{Contains, Get};

	let who = || -> pezsp_runtime::MultiAddress<pezsp_runtime::AccountId32, ()> {
		pezsp_runtime::MultiAddress::Id(pezsp_runtime::AccountId32::new([0u8; 32]))
	};
	let who = || -> pezsp_runtime::MultiAddress<pezsp_runtime::AccountId32, ()> {
		pezsp_runtime::MultiAddress::Id(pezsp_runtime::AccountId32::new([0u8; 32]))
	};
	let pez = PezAssetId::get();
	let other = pez + 1;

	let calls = |id: u32| -> Vec<(&'static str, RuntimeCall)> {
		vec![
			(
				"force_asset_status",
				RuntimeCall::Assets(pezpallet_assets::Call::force_asset_status {
					id: id.into(),
					owner: who(),
					issuer: who(),
					admin: who(),
					freezer: who(),
					min_balance: 1,
					is_sufficient: false,
					is_frozen: false,
				}),
			),
			(
				"start_destroy",
				RuntimeCall::Assets(pezpallet_assets::Call::start_destroy { id: id.into() }),
			),
			(
				"mint",
				RuntimeCall::Assets(pezpallet_assets::Call::mint {
					id: id.into(),
					beneficiary: who(),
					amount: 1,
				}),
			),
		]
	};

	for (name, call) in calls(pez) {
		assert!(!NoTouchingPez::contains(&call), "XCM can still reach `{name}` on PEZ");
	}
	for (name, call) in calls(other) {
		assert!(
			NoTouchingPez::contains(&call),
			"`{name}` on another asset was refused; the filter names the asset, not the call"
		);
	}
}

/// HEZ's rate is policy, its ceiling and its base are not.
///
/// The two knobs moved from the binary into storage so the franchise that bears them can turn
/// them; the ceiling stayed compiled in, because a ceiling the same body could raise is not a
/// ceiling.
mod hez_parameters {
	use asset_hub_zagros_runtime::{
		dynamic_params::hez, staking::MAX_INFLATION_RATE, Runtime, RuntimeOrigin,
	};
	use pezframe_support::{assert_noop, assert_ok, traits::Get};
	use pezpallet_staking_async::EraPayout as _;
	use pezsp_runtime::{traits::BadOrigin, BuildStorage, Perbill};

	fn new_test_ext() -> pezsp_io::TestExternalities {
		pezframe_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into()
	}

	const YEAR_MS: u64 = (1000 * 3600 * 24 * 36525) / 100;

	fn yearly_payout() -> (u128, u128) {
		<asset_hub_zagros_runtime::staking::EraPayout as pezpallet_staking_async::EraPayout<
			u128,
		>>::era_payout(0, 0, YEAR_MS)
	}

	#[test]
	fn the_defaults_are_what_the_constants_were() {
		new_test_ext().execute_with(|| {
			assert_eq!(hez::InflationRate::get(), Perbill::from_percent(8));
			assert_eq!(hez::TreasuryShare::get(), Perbill::from_percent(15));

			// 8% of 200M, split 85/15. Moving these to storage must not have moved the money.
			let (stakers, treasury) = yearly_payout();
			let emission = 16_000_000_000_000_000_000u128;
			assert_eq!(treasury, Perbill::from_percent(15).mul_floor(emission));
			assert_eq!(stakers + treasury, emission);
		});
	}

	#[test]
	fn the_economic_franchise_turns_them_and_nobody_else_does() {
		new_test_ext().execute_with(|| {
			let raise = |o: RuntimeOrigin| {
				pezpallet_parameters::Pezpallet::<Runtime>::set_parameter(
					o,
					asset_hub_zagros_runtime::RuntimeParameters::Hez(
						hez::Parameters::InflationRate(
							hez::InflationRate,
							Some(Perbill::from_percent(9)),
						),
					),
				)
			};

			// This chain has no root track, and Root is not the body that bears dilution.
			assert_noop!(raise(RuntimeOrigin::root()), BadOrigin);
			assert_noop!(raise(RuntimeOrigin::signed([1u8; 32].into())), BadOrigin);

			assert_ok!(raise(asset_hub_zagros_runtime::governance::pezpallet_custom_origins::Origin::EconomicAdmin.into()));
			assert_eq!(hez::InflationRate::get(), Perbill::from_percent(9));
			assert_eq!(yearly_payout().0 + yearly_payout().1, 18_000_000_000_000_000_000);
		});
	}

	#[test]
	fn the_ceiling_holds_whatever_the_parameter_says() {
		new_test_ext().execute_with(|| {
			assert_ok!(pezpallet_parameters::Pezpallet::<Runtime>::set_parameter(
				asset_hub_zagros_runtime::governance::pezpallet_custom_origins::Origin::EconomicAdmin.into(),
				asset_hub_zagros_runtime::RuntimeParameters::Hez(hez::Parameters::InflationRate(
					hez::InflationRate,
					Some(Perbill::from_percent(90)),
				)),
			));

			// The parameter took the value; the payout did not.
			assert_eq!(hez::InflationRate::get(), Perbill::from_percent(90));
			let (stakers, treasury) = yearly_payout();
			assert_eq!(
				stakers + treasury,
				MAX_INFLATION_RATE.mul_floor(200_000_000_000_000_000_000u128)
			);
		});
	}
}
