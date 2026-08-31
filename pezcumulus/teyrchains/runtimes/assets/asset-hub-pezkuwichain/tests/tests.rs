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

//! Tests for the Pezkuwichain Assets Hub chain.

extern crate alloc;

use asset_hub_pezkuwichain_runtime::{
	xcm_config,
	xcm_config::{
		bridging, CheckingAccount, GovernanceLocation, LocationToAccountId, StakingPot,
		TokenLocation, TrustBackedAssetsPalletLocation, XcmConfig,
	},
	AllPalletsWithoutSystem, AssetConversion, AssetDeposit, Assets, Balances, Block,
	CollatorSelection, ExistentialDeposit, ForeignAssets, ForeignAssetsInstance,
	MetadataDepositBase, MetadataDepositPerByte, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin,
	SessionKeys, TeyrchainSystem, ToZagrosXcmRouterInstance, TrustBackedAssetsInstance, XcmpQueue,
};
use asset_test_pezutils::{
	test_cases_over_bridge::TestBridgingConfig, CollatorSessionKey, CollatorSessionKeys,
	ExtBuilder, GovernanceOrigin, SlotDurations,
};
use codec::{Decode, Encode};
use hex_literal::hex;
use pezframe_support::{
	assert_ok, parameter_types,
	traits::{
		fungible::{Inspect, Mutate},
		fungibles::{
			Create, Inspect as FungiblesInspect, InspectEnumerable, Mutate as FungiblesMutate,
		},
	},
	weights::{Weight, WeightToFee as WeightToFeeT},
};
use pezsp_consensus_aura::SlotDuration;
use pezsp_core::crypto::Ss58Codec;
use pezsp_runtime::traits::MaybeEquivalence;
use std::convert::Into;
use testnet_teyrchains_constants::pezkuwichain::{consensus::*, currency::UNITS, fee::WeightToFee};
use teyrchains_common::{AccountId, AssetIdForTrustBackedAssets, AuraId, Balance};
use xcm::latest::{
	prelude::{Assets as XcmAssets, *},
	ZAGROS_GENESIS_HASH,
};
use xcm_builder::WithLatestLocationConverter;
use xcm_executor::traits::{JustTry, TransactAsset, WeightTrader};
use xcm_runtime_pezapis::conversions::LocationToAccountHelper;

const ALICE: [u8; 32] = [1u8; 32];
const SOME_ASSET_ADMIN: [u8; 32] = [5u8; 32];

parameter_types! {
	pub Governance: GovernanceOrigin<RuntimeOrigin> = GovernanceOrigin::Location(GovernanceLocation::get());
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

#[test]
fn test_buy_and_refund_weight_in_native() {
	ExtBuilder::<Runtime>::default()
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
				Box::new(Location::try_from(native_location.clone()).expect("conversion works")),
				Box::new(Location::try_from(asset_1_location.clone()).expect("conversion works"))
			));

			assert_ok!(AssetConversion::add_liquidity(
				RuntimeHelper::origin_of(bob.clone()),
				Box::new(Location::try_from(native_location.clone()).expect("conversion works")),
				Box::new(Location::try_from(asset_1_location.clone()).expect("conversion works")),
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
				Location::try_from(native_location).expect("conversion works"),
				Location::try_from(asset_1_location.clone()).expect("conversion works"),
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
				Location::try_from(TokenLocation::get()).expect("conversion works");
			let foreign_location = Location {
				parents: 1,
				interior: (Junction::Teyrchain(1234), Junction::GeneralIndex(12345)).into(),
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

			// Mint the asset to alice so the withdraw below has something to take; at least
			// ED, or the mint itself is rejected.
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
fn test_asset_xcm_trader_not_possible_for_non_sufficient_assets() {
	ExtBuilder::<Runtime>::default()
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
			let alice_location: Location =
				Junction::AccountId32 { network: None, id: ALICE.into() }.into();
			let asset_holding =
				<XcmConfig as xcm_executor::Config>::AssetTransactor::withdraw_asset(
					&asset,
					&alice_location,
					Some(&ctx),
				)
				.expect("Failed to withdraw asset");

			// Make sure again buy_weight does return an error, handing the asset back.
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

#[test]
fn test_assets_balances_api_works() {
	use pez_assets_common::runtime_api::runtime_decl_for_fungibles_api::FungiblesApi;

	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(pezsp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			let local_asset_id = 1;
			let foreign_asset_id_location =
				Location::new(1, [Junction::Teyrchain(1234), Junction::GeneralIndex(12345)]);

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
			assert!(result.inner().iter().any(|asset| asset.eq(
				&pez_assets_common::fungible_conversion::convert_balance::<TokenLocation, Balance>(
					some_currency
				)
				.unwrap()
			)));
			// check trusted asset
			assert!(result.inner().iter().any(|asset| asset.eq(&(
				AssetIdForTrustBackedAssetsConvert::convert_back(&local_asset_id).unwrap(),
				minimum_asset_balance
			)
				.into())));
			// check foreign asset
			assert!(result.inner().iter().any(|asset| asset.eq(&(
				WithLatestLocationConverter::<Location>::convert_back(&foreign_asset_id_location)
					.unwrap(),
				6 * foreign_asset_minimum_asset_balance
			)
				.into())));
		});
}

asset_test_pezutils::include_teleports_for_native_asset_works!(
	Runtime,
	AllPalletsWithoutSystem,
	XcmConfig,
	(),
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
	Location,
	JustTry,
	collator_session_keys(),
	ExistentialDeposit::get(),
	Location::new(1, [Junction::Teyrchain(1313), Junction::GeneralIndex(12345)]),
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
	Location,
	WithLatestLocationConverter<Location>,
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

fn limited_reserve_transfer_assets_for_native_asset_over_bridge_works(
	bridging_configuration: fn() -> TestBridgingConfig,
) {
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
		bridging_configuration,
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

mod asset_hub_pezkuwichain_tests {
	use super::*;
	use asset_hub_pezkuwichain_runtime::{ForeignAssetReserveData, PezkuwiXcm};
	use xcm::latest::ZAGROS_GENESIS_HASH;
	use xcm_executor::traits::ConvertLocation;

	fn bridging_to_asset_hub_zagros() -> TestBridgingConfig {
		let _ = PezkuwiXcm::force_xcm_version(
			RuntimeOrigin::root(),
			Box::new(bridging::to_zagros::AssetHubZagros::get()),
			XCM_VERSION,
		)
		.expect("version saved!");
		TestBridgingConfig {
			bridged_network: bridging::to_zagros::ZagrosNetwork::get(),
			local_bridge_hub_para_id: bridging::SiblingBridgeHubParaId::get(),
			local_bridge_hub_location: bridging::SiblingBridgeHub::get(),
			bridged_target_location: bridging::to_zagros::AssetHubZagros::get(),
		}
	}

	#[test]
	fn limited_reserve_transfer_assets_for_native_asset_to_asset_hub_zagros_works() {
		limited_reserve_transfer_assets_for_native_asset_over_bridge_works(
			bridging_to_asset_hub_zagros,
		)
	}

	#[test]
	fn receive_reserve_asset_deposited_wnd_from_asset_hub_zagros_fees_paid_by_pool_swap_works() {
		const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
		let block_author_account = AccountId::from(BLOCK_AUTHOR_ACCOUNT);
		let staking_pot = StakingPot::get();

		let foreign_asset_id_location =
			Location::new(2, [GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH))]);
		let reserve_location =
			Location::new(2, [GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)), Teyrchain(1000)]);
		let foreign_asset_reserve_data =
			ForeignAssetReserveData { reserve: reserve_location, teleportable: false };
		let foreign_asset_id_minimum_balance = 1_000_000_000;
		// sovereign account as foreign asset owner (can be whoever for this scenario)
		let foreign_asset_owner =
			LocationToAccountId::convert_location(&Location::parent()).unwrap();
		let foreign_asset_create_params = (
			foreign_asset_owner.clone(),
			foreign_asset_id_location.clone(),
			foreign_asset_reserve_data,
			foreign_asset_id_minimum_balance,
		);
		let pool_params = (
			foreign_asset_owner,
			foreign_asset_id_location.clone(),
			foreign_asset_id_minimum_balance,
		);

		asset_test_pezutils::test_cases_over_bridge::receive_reserve_asset_deposited_from_different_consensus_works::<
			Runtime,
			AllPalletsWithoutSystem,
			XcmConfig,
			ForeignAssetsInstance,
		>(
			collator_session_keys().add(collator_session_key(BLOCK_AUTHOR_ACCOUNT)),
			ExistentialDeposit::get(),
			AccountId::from([73; 32]),
			block_author_account,
			// receiving WNDs
			foreign_asset_create_params,
			1000000000000,
			|| {
				// setup pool for paying fees to touch `SwapFirstAssetTrader`
				asset_test_pezutils::test_cases::setup_pool_for_paying_fees_with_foreign_assets::<Runtime, RuntimeOrigin>(ExistentialDeposit::get(), pool_params);
				// staking pot account for collecting local native fees from `BuyExecution`
				let _ = Balances::force_set_balance(RuntimeOrigin::root(), StakingPot::get().into(), ExistentialDeposit::get());
				// prepare bridge configuration
				bridging_to_asset_hub_zagros()
			},
			(
				[PalletInstance(pezbp_bridge_hub_pezkuwichain::WITH_BRIDGE_PEZKUWICHAIN_TO_ZAGROS_MESSAGES_PALLET_INDEX)].into(),
				GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
				[xcm::latest::Junction::Teyrchain(1000)].into()
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
	fn receive_reserve_asset_deposited_wnd_from_asset_hub_zagros_fees_paid_by_sufficient_asset_works(
	) {
		const BLOCK_AUTHOR_ACCOUNT: [u8; 32] = [13; 32];
		let block_author_account = AccountId::from(BLOCK_AUTHOR_ACCOUNT);
		let staking_pot = StakingPot::get();

		let foreign_asset_id_location =
			Location::new(2, [GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH))]);
		let reserve_location =
			Location::new(2, [GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)), Teyrchain(1000)]);
		let foreign_asset_reserve_data =
			ForeignAssetReserveData { reserve: reserve_location, teleportable: false };
		let foreign_asset_id_minimum_balance = 1_000_000_000;
		// sovereign account as foreign asset owner (can be whoever for this scenario)
		let foreign_asset_owner =
			LocationToAccountId::convert_location(&Location::parent()).unwrap();
		let foreign_asset_create_params = (
			foreign_asset_owner.clone(),
			foreign_asset_id_location.clone(),
			foreign_asset_reserve_data,
			foreign_asset_id_minimum_balance,
		);
		let pool_params = (
			foreign_asset_owner,
			foreign_asset_id_location.clone(),
			foreign_asset_id_minimum_balance,
		);

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
			// receiving WNDs
			foreign_asset_create_params,
			1000000000000,
			|| {
				asset_test_pezutils::test_cases::setup_pool_for_paying_fees_with_foreign_assets::<Runtime, RuntimeOrigin>(ExistentialDeposit::get(), pool_params);
				bridging_to_asset_hub_zagros()
			},
			(
				[PalletInstance(pezbp_bridge_hub_pezkuwichain::WITH_BRIDGE_PEZKUWICHAIN_TO_ZAGROS_MESSAGES_PALLET_INDEX)].into(),
				GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
				[xcm::latest::Junction::Teyrchain(1000)].into()
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
	fn report_bridge_status_from_xcm_bridge_router_for_zagros_works() {
		asset_test_pezutils::test_cases_over_bridge::report_bridge_status_from_xcm_bridge_router_works::<
			Runtime,
			AllPalletsWithoutSystem,
			XcmConfig,
			LocationToAccountId,
			ToZagrosXcmRouterInstance,
		>(
			collator_session_keys(),
			bridging_to_asset_hub_zagros,
			|| pezbp_asset_hub_pezkuwichain::build_congestion_message(Default::default(), true).into(),
			|| {
				pezbp_asset_hub_pezkuwichain::build_congestion_message(Default::default(), false)
					.into()
			},
		)
	}

	#[test]
	fn test_report_bridge_status_call_compatibility() {
		// if this test fails, make sure `pezbp_asset_hub_pezkuwichain` has valid encoding
		assert_eq!(
			RuntimeCall::ToZagrosXcmRouter(
				pezpallet_xcm_bridge_hub_router::Call::report_bridge_status {
					bridge_id: Default::default(),
					is_congested: true,
				}
			)
			.encode(),
			pezbp_asset_hub_pezkuwichain::Call::ToZagrosXcmRouter(
				pezbp_asset_hub_pezkuwichain::XcmBridgeHubRouterCall::report_bridge_status {
					bridge_id: Default::default(),
					is_congested: true,
				}
			)
			.encode()
		);
	}

	#[test]
	fn check_sane_weight_report_bridge_status_for_zagros() {
		use pezpallet_xcm_bridge_hub_router::WeightInfo;
		let actual = <Runtime as pezpallet_xcm_bridge_hub_router::Config<
			ToZagrosXcmRouterInstance,
		>>::WeightInfo::report_bridge_status();
		let max_weight =
			pezbp_asset_hub_pezkuwichain::XcmBridgeHubRouterTransactCallMaxWeight::get();
		assert!(
			actual.all_lte(max_weight),
			"max_weight: {:?} should be adjusted to actual {:?}",
			max_weight,
			actual
		);
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
fn change_xcm_bridge_hub_ethereum_base_fee_by_governance_works() {
	asset_test_pezutils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		bridging::to_ethereum::BridgeHubEthereumBaseFee,
		Balance,
	>(
		collator_session_keys(),
		1000,
		Governance::get(),
		|| {
			tracing::error!(
				target: "bridges::estimate",
				actual_value=%bridging::to_ethereum::BridgeHubEthereumBaseFee::get(),
				runtime=%<Runtime as pezframe_system::Config>::Version::get(),
				"`bridging::BridgeHubEthereumBaseFee`"
			);
			(
				bridging::to_ethereum::BridgeHubEthereumBaseFee::key().to_vec(),
				bridging::to_ethereum::BridgeHubEthereumBaseFee::get(),
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
fn location_conversion_works() {
	// The expected accounts are derived, not chosen: `blake2_256` over the location's standard
	// description. The Zagros ones therefore move with `ZAGROS_GENESIS_HASH`, which is still
	// the launch placeholder of all zeroes -- when the real hash is set they all change, and
	// this test is what will say so. Regenerate them from the failure output; do not hand-edit
	// one and leave the rest.
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
				[
					xcm::latest::Junction::Teyrchain(1111),
					AccountKey20 { network: None, key: [0u8; 20] },
				],
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
				[
					xcm::latest::Junction::Teyrchain(1111),
					Plurality { id: BodyId::Treasury, part: BodyPart::Voice },
				],
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
				[
					xcm::latest::Junction::Teyrchain(1111),
					Plurality { id: BodyId::Unit, part: BodyPart::Voice },
				],
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
			description: "Describe Zagros Location",
			location: Location::new(2, [GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH))]),
			expected_account_id_str: "5GLzMCt7Y59gpYxwuuHk9jJpuzm5k72j7KYYt7uKWkFUbKN3",
		},
		TestCase {
			description: "Describe Zagros AccountID",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					AccountId32 { network: None, id: AccountId::from(ALICE).into() },
				],
			),
			expected_account_id_str: "5HXrf6D64DkCsfy6NjQ6yszkTjM6syymGQPRcwiHsWazPRMj",
		},
		TestCase {
			description: "Describe Zagros AccountKey",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					AccountKey20 { network: None, key: [0u8; 20] },
				],
			),
			expected_account_id_str: "5CtpmbSqTRhn5UP9YYJUaZBqScmcze1yAzerTSTKtU2qA75m",
		},
		TestCase {
			description: "Describe Zagros Treasury Plurality",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					Plurality { id: BodyId::Treasury, part: BodyPart::Voice },
				],
			),
			expected_account_id_str: "5GA4VgZ19uBK7Yaj5UGSw2yVURE6x1V9yd6hkvpw9KD7yK2G",
		},
		TestCase {
			description: "Describe Zagros Teyrchain Location",
			location: Location::new(
				2,
				[GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)), Teyrchain(1000)],
			),
			expected_account_id_str: "5Hk6aXbnUHUMeuWwN7LLy7NSb3SCMNExMwLDovsTezhRuRqS",
		},
		TestCase {
			description: "Describe Zagros Teyrchain AccountID",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					Teyrchain(1000),
					AccountId32 { network: None, id: AccountId::from(ALICE).into() },
				],
			),
			expected_account_id_str: "5CbgDcpiCPZDp5XvhQ5ioVpaaMcWrJ53sm6LuzXDYHSpm7Ds",
		},
		TestCase {
			description: "Describe Zagros Teyrchain AccountKey",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					Teyrchain(1000),
					AccountKey20 { network: None, key: [0u8; 20] },
				],
			),
			expected_account_id_str: "5CopacobxcMvQwyX3kT999BAqrR3WntS7cti45JD5KaY7Zup",
		},
		TestCase {
			description: "Describe Zagros Teyrchain Treasury Plurality",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					Teyrchain(1000),
					Plurality { id: BodyId::Treasury, part: BodyPart::Voice },
				],
			),
			expected_account_id_str: "5FAbFYKCLMf4JYbhTLPpxTkWYzzpRZs1k6gubbAkErY21Nde",
		},
		TestCase {
			description: "Describe Zagros USDT Location",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					Teyrchain(1000),
					PalletInstance(50),
					GeneralIndex(1984),
				],
			),
			expected_account_id_str: "5F4KnP35Jy8H4tBzuSw1eMjuMMBkMgYaczuqifHEsEYUFV7E",
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
	>(ExistentialDeposit::get(), ZAGROS_GENESIS_HASH);
}

/// The register decides and the treasury pays, so a spend voted on People has to reach the
/// Asset Hub. Two gates stand in the way and only one of them was open: `WaivedLocations`
/// already charges a sibling system chain nothing, but the barrier runs first and named only
/// the relay, its pluralities, the relay treasury and the Bridge Hub -- so the
/// message `welati::send_government_spend` builds was refused before the fee policy was ever
/// consulted, and the origin check it was written against was never reached.
///
/// Neither side of that pair is exercised by the runtime's other tests, and the pallet's own
/// tests use a mock sender, so nothing caught it.
#[test]
fn people_may_execute_unpaid_on_this_asset_hub() {
	use pezframe_support::traits::Contains;
	use testnet_teyrchains_constants::pezkuwichain::locations::PeopleLocation;
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

/// `welati` builds the emission call by hand, and nothing fails if the bytes drift.
///
/// The Treasurer's call lives on the People chain, which cannot name this runtime's types, so
/// `send_emission_rate` writes the address itself: pallet index, call index, then the two
/// variant indices that reach `InflationRate` inside `RuntimeParameters`. If any of the four
/// moves, the `Transact` stops decoding -- silently, because the sending chain has already
/// recorded the change and emitted its event. This pins the bytes it has to produce, exactly
/// as `the_treasury_call_encodes_the_way_welati_builds_it` does for the spending call.
#[test]
fn the_emission_call_encodes_the_way_welati_builds_it() {
	use asset_hub_pezkuwichain_runtime::dynamic_params::hez;
	use codec::Encode;

	let rate = pezsp_runtime::Perbill::from_percent(9);

	let real = RuntimeCall::Parameters(pezpallet_parameters::Call::<Runtime>::set_parameter {
		key_value: asset_hub_pezkuwichain_runtime::RuntimeParameters::Hez(
			hez::Parameters::InflationRate(hez::InflationRate, Some(rate)),
		),
	})
	.encode();

	// Exactly what `welati::send_emission_rate` puts on the wire: pallet 79, call 0, then
	// `Hez` (0), `InflationRate` (0), and the value.
	let by_hand = (79u8, 0u8, 0u8, 0u8, Some(rate)).encode();

	assert_eq!(real, by_hand, "welati builds an emission call this runtime cannot decode");
}

// `state_and_economic_origins_do_not_overlap` stood here and moved to the emulated tests.
//
// It compared this chain's track names against a *hardcoded copy* of the register's three, and
// never read the relay at all. A sentinel holding a copy of the thing it guards goes stale the
// first time the original changes -- rename a track on People and this one keeps passing over a
// list nobody updated. The emulated crate can see all three runtimes, so the version there reads
// the real lists instead of remembering them.

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
	use asset_hub_pezkuwichain_runtime::{xcm_config::NoTouchingPez, PezAssetId, RuntimeCall};
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
	use asset_hub_pezkuwichain_runtime::{
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

	/// The People chain speaking as itself -- what `welati::set_emission_rate` produces after
	/// the Treasurer's tiki has been checked over there. This chain trusts the register's chain
	/// and does not re-check which office sent it, exactly as it does for `spend_budget`.
	fn people_origin() -> RuntimeOrigin {
		// `EnsureXcm` reads `pezpallet_xcm::Origin::Xcm(location)` -- the location an incoming
		// message was converted from -- not the sibling-teyrchain origin. Those are two
		// different things and only the first one carries where the message came from.
		pezpallet_xcm::Origin::Xcm(xcm::latest::Location::new(
			1,
			[xcm::latest::Junction::Teyrchain(
				pezkuwichain_runtime_constants::system_teyrchain::PEOPLE_ID,
			)],
		))
		.into()
	}

	fn yearly_payout() -> (u128, u128) {
		<asset_hub_pezkuwichain_runtime::staking::EraPayout as pezpallet_staking_async::EraPayout<
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
	fn the_people_chain_turns_them_and_nobody_else_does() {
		new_test_ext().execute_with(|| {
			let raise = |o: RuntimeOrigin| {
				pezpallet_parameters::Pezpallet::<Runtime>::set_parameter(
					o,
					asset_hub_pezkuwichain_runtime::RuntimeParameters::Hez(
						hez::Parameters::InflationRate(
							hez::InflationRate,
							Some(Perbill::from_percent(9)),
						),
					),
				)
			};

			// Neither Root nor a signed account: the rate is the Treasurer's, and the Treasurer
			// is an office on the People chain. What arrives here is that chain's message.
			assert_noop!(raise(RuntimeOrigin::root()), BadOrigin);
			assert_noop!(raise(RuntimeOrigin::signed([1u8; 32].into())), BadOrigin);

			assert_ok!(raise(people_origin()));
			assert_eq!(hez::InflationRate::get(), Perbill::from_percent(9));
			assert_eq!(yearly_payout().0 + yearly_payout().1, 18_000_000_000_000_000_000);
		});
	}

	#[test]
	fn the_ceiling_holds_whatever_the_parameter_says() {
		new_test_ext().execute_with(|| {
			assert_ok!(pezpallet_parameters::Pezpallet::<Runtime>::set_parameter(
				people_origin(),
				asset_hub_pezkuwichain_runtime::RuntimeParameters::Hez(
					hez::Parameters::InflationRate(
						hez::InflationRate,
						Some(Perbill::from_percent(90)),
					)
				),
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
