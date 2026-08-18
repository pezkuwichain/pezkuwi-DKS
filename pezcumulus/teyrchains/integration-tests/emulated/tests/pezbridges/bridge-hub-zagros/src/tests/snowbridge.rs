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
use crate::{
	imports::{pez_penpal_emulated_chain::pez_penpal_runtime::xcm_config::CheckingAccount, *},
	tests::{
		assert_bridge_hub_pezkuwichain_message_received, assert_bridge_hub_zagros_message_accepted,
		asset_hub_pezkuwichain_location, asset_hub_zagros_global_location,
		bridged_roc_at_ah_zagros, bridged_wnd_at_ah_pezkuwichain,
		create_foreign_on_ah_pezkuwichain, create_foreign_on_ah_zagros,
		pez_penpal_emulated_chain::pez_penpal_runtime,
		snowbridge_common::{bridge_hub, ethereum, pezsnowbridge_sovereign, register_roc_on_bh},
	},
};
use asset_hub_zagros_runtime::xcm_config::{
	bridging::to_ethereum::DefaultBridgeHubEthereumBaseFee,
	UniversalLocation as AssetHubZagrosUniversalLocation,
};
use codec::Encode;
use emulated_integration_tests_common::{
	snowbridge::{SEPOLIA_ID, WETH},
	PENPAL_B_ID, RESERVABLE_ASSET_ID,
};
use hex_literal::hex;
use pezbridge_hub_zagros_runtime::{
	bridge_to_ethereum_config::EthereumGatewayAddress, EthereumBeaconClient, EthereumInboundQueue,
};
use pezframe_support::traits::fungibles::Mutate;
use pezkuwichain_zagros_system_emulated_network::{
	asset_hub_zagros_emulated_chain::genesis::AssetHubZagrosAssetOwner,
	pez_penpal_emulated_chain::PARA_ID_B, zagros_emulated_chain::zagros_runtime::Dmp,
};
use pezsnowbridge_core::{AssetMetadata, TokenIdOf};
use pezsnowbridge_inbound_queue_primitives::{
	v1::{Command, Destination, MessageV1, VersionedMessage},
	EventFixture,
};
use pezsnowbridge_pezpallet_inbound_queue_fixtures::send_native_eth::make_send_native_eth_message;
use pezsp_core::{H160, H256};
use testnet_teyrchains_constants::zagros::snowbridge::EthereumNetwork;
use xcm_builder::ExternalConsensusLocationsConverterFor;
use xcm_executor::traits::ConvertLocation;

const INITIAL_FUND: u128 = 5_000_000_000_000;
const ETHEREUM_DESTINATION_ADDRESS: [u8; 20] = hex!("44a57ee2f2FCcb85FDa2B0B18EBD0D8D2333700e");
const XCM_FEE: u128 = 100_000_000_000;
const INSUFFICIENT_XCM_FEE: u128 = 1000;
const TOKEN_AMOUNT: u128 = 100_000_000_000;
const BRIDGE_FEE: u128 = 4_000_000_000_000;

pub fn send_inbound_message(fixture: EventFixture) -> DispatchResult {
	EthereumBeaconClient::store_finalized_header(
		fixture.finalized_header,
		fixture.block_roots_root,
	)
	.unwrap();
	EthereumInboundQueue::submit(
		BridgeHubZagrosRuntimeOrigin::signed(BridgeHubZagrosSender::get()),
		fixture.event,
	)
}

/// Tests the registering of a token as an asset on AssetHub.
#[test]
fn register_token_from_ethereum_to_asset_hub() {
	// Fund AssetHub sovereign account so that it can pay execution fees.
	BridgeHubZagros::fund_para_sovereign(AssetHubZagros::para_id().into(), INITIAL_FUND);
	// Fund Snowbridge Sovereign to satisfy ED.
	AssetHubZagros::fund_accounts(vec![(pezsnowbridge_sovereign(), INITIAL_FUND)]);

	let token = H160::random();

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::RegisterToken { token: token.into(), fee: XCM_FEE },
		});
		let (xcm, _) = EthereumInboundQueue::do_convert([0; 32].into(), message).unwrap();
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id().into()).unwrap();

		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Created { .. }) => {},]
		);
	});
}

/// Tests the registering of a token as an asset on AssetHub, and then subsequently sending
/// a token from Ethereum to AssetHub.
#[test]
fn send_weth_token_from_ethereum_to_asset_hub() {
	let ethereum_sovereign: AccountId = pezsnowbridge_sovereign();

	BridgeHubZagros::fund_para_sovereign(AssetHubZagros::para_id().into(), INITIAL_FUND);

	// Fund ethereum sovereign on AssetHub
	AssetHubZagros::fund_accounts(vec![
		(AssetHubZagrosReceiver::get(), INITIAL_FUND),
		(ethereum_sovereign, INITIAL_FUND),
	]);

	// Send the token
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		type EthereumInboundQueue =
			<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumInboundQueue;
		let message_id: H256 = [0; 32].into();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::AccountId32 { id: AssetHubZagrosSender::get().into() },
				amount: TOKEN_AMOUNT,
				fee: XCM_FEE,
			},
		});
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		assert_ok!(EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id().into()));

		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},
			]
		);
	});
}

/// Tests sending a token to a 3rd party teyrchain, called PenPal. The token reserve is
/// still located on AssetHub.
#[test]
fn send_weth_from_ethereum_to_penpal() {
	let asset_hub_sovereign = BridgeHubZagros::sovereign_account_id_of(Location::new(
		1,
		[Teyrchain(AssetHubZagros::para_id().into())],
	));
	// Fund AssetHub sovereign account so it can pay execution fees for the asset transfer
	BridgeHubZagros::fund_accounts(vec![(asset_hub_sovereign.clone(), INITIAL_FUND)]);

	// Fund PenPal receiver (covering ED)
	let native_id: Location = Parent.into();
	let receiver: AccountId = [
		28, 189, 45, 67, 83, 10, 68, 112, 90, 208, 136, 175, 49, 62, 24, 248, 11, 83, 239, 22, 179,
		97, 119, 205, 75, 119, 184, 70, 242, 165, 240, 124,
	]
	.into();
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		native_id,
		receiver,
		pez_penpal_runtime::EXISTENTIAL_DEPOSIT,
	);

	PenpalB::execute_with(|| {
		let key = PenpalCustomizableAssetFromSystemAssetHub::key().to_vec();
		let value = Location::new(2, [GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })]).encode();
		assert_eq!(key, hex!("770800eb78be69c327d8334d09276072"));
		assert_eq!(value, hex!("020109079edaa802"));
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })]).encode(),
			)],
		));
	});

	let ethereum_network_v5: NetworkId = EthereumNetwork::get().into();

	// The Weth asset location, identified by the contract address on Ethereum
	let weth_asset_location: Location =
		(Parent, Parent, ethereum_network_v5, AccountKey20 { network: None, key: WETH }).into();

	// Fund ethereum sovereign on AssetHub
	let ethereum_sovereign: AccountId = pezsnowbridge_sovereign();
	AssetHubZagros::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);

	// Create asset on the Penpal teyrchain.
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::force_create(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			weth_asset_location.clone(),
			asset_hub_sovereign.into(),
			false,
			1000,
		));

		assert!(<PenpalB as PenpalBPallet>::ForeignAssets::asset_exists(weth_asset_location));
	});

	// Send the token
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		type EthereumInboundQueue =
			<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumInboundQueue;
		let message_id: H256 = [0; 32].into();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::ForeignAccountId32 {
					para_id: PENPAL_B_ID,
					id: PenpalBReceiver::get().into(),
					fee: XCM_FEE,
				},
				amount: TOKEN_AMOUNT,
				fee: XCM_FEE,
			},
		});
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		assert_ok!(EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id().into()));

		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		// Check that the assets were issued on AssetHub
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},
				RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	PenpalB::execute_with(|| {
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;
		// Check that the assets were issued on PenPal
		assert_expected_events!(
			PenpalB,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},
			]
		);
	});
}

/// Tests the full cycle of eth transfers:
/// - sending a token to AssetHub
/// - returning the token to Ethereum
#[test]
fn send_eth_asset_from_asset_hub_to_ethereum_and_back() {
	let ethereum_network: NetworkId = EthereumNetwork::get().into();
	let origin_location: Location = (Parent, Parent, ethereum_network).into();

	use asset_hub_zagros_runtime::xcm_config::bridging::to_ethereum::DefaultBridgeHubEthereumBaseFee;
	let assethub_location = BridgeHubZagros::sibling_location_of(AssetHubZagros::para_id());
	let assethub_sovereign = BridgeHubZagros::sovereign_account_id_of(assethub_location);
	let ethereum_sovereign: AccountId = pezsnowbridge_sovereign();

	AssetHubZagros::force_default_xcm_version(Some(XCM_VERSION));
	BridgeHubZagros::force_default_xcm_version(Some(XCM_VERSION));
	AssetHubZagros::force_xcm_version(origin_location.clone(), XCM_VERSION);

	BridgeHubZagros::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);
	AssetHubZagros::fund_accounts(vec![
		(AssetHubZagrosReceiver::get(), INITIAL_FUND),
		(ethereum_sovereign.clone(), INITIAL_FUND),
	]);

	const ETH_AMOUNT: u128 = 1_000_000_000_000_000_000;

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		type RuntimeOrigin = <BridgeHubZagros as Chain>::RuntimeOrigin;

		// Set the gateway. This is needed because new fixtures use a different gateway address.
		assert_ok!(<BridgeHubZagros as Chain>::System::set_storage(
			RuntimeOrigin::root(),
			vec![(
				EthereumGatewayAddress::key().to_vec(),
				pezsp_core::H160(hex!("87d1f7fdfEe7f651FaBc8bFCB6E086C278b77A7d")).encode(),
			)],
		));

		// Construct SendToken message and sent to inbound queue
		assert_ok!(send_inbound_message(make_send_native_eth_message()));

		// Check that the send token message was sent using xcm
		assert_expected_events!(
			BridgeHubZagros,
			vec![
				RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;

		let _issued_event = RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited {
			asset_id: origin_location.clone(),
			who: AssetHubZagrosReceiver::get().into(),
			amount: ETH_AMOUNT,
		});
		// Check that AssetHub has issued the foreign asset
		assert_expected_events!(
			AssetHubZagros,
			vec![
				_issued_event => {},
			]
		);
		let assets =
			vec![Asset { id: AssetId(origin_location.clone()), fun: Fungible(ETH_AMOUNT) }];
		let multi_assets = VersionedAssets::from(Assets::from(assets.clone()));

		let destination = origin_location.clone().into();

		let beneficiary = VersionedLocation::from(Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		));

		let free_balance_before = <AssetHubZagros as AssetHubZagrosPallet>::Balances::free_balance(
			AssetHubZagrosReceiver::get(),
		);
		// Send the Weth back to Ethereum
		let fee_asset_id: AssetId = AssetId(origin_location.clone());
		<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::limited_reserve_transfer_assets(
			RuntimeOrigin::signed(AssetHubZagrosReceiver::get()),
			Box::new(destination),
			Box::new(beneficiary),
			Box::new(multi_assets),
			fee_asset_index(&Assets::from(assets.clone()), &fee_asset_id),
			Unlimited,
		)
		.unwrap();

		let _burned_event = RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Withdrawn {
			asset_id: origin_location.clone(),
			who: AssetHubZagrosReceiver::get().into(),
			amount: ETH_AMOUNT,
		});
		// Check that AssetHub has issued the foreign asset
		let _destination = origin_location.clone();
		assert_expected_events!(
			AssetHubZagros,
			vec![
				_burned_event => {},
				RuntimeEvent::PezkuwiXcm(pezpallet_xcm::Event::Sent {
					destination: _destination, ..
				}) => {},
			]
		);

		let free_balance_after = <AssetHubZagros as AssetHubZagrosPallet>::Balances::free_balance(
			AssetHubZagrosReceiver::get(),
		);
		// Assert at least DefaultBridgeHubEthereumBaseFee charged from the sender
		let free_balance_diff = free_balance_before - free_balance_after;
		assert!(free_balance_diff > DefaultBridgeHubEthereumBaseFee::get());
	});

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubZagros,
			vec![
				RuntimeEvent::EthereumOutboundQueue(pezsnowbridge_pezpallet_outbound_queue::Event::MessageAccepted {..}) => {},
				RuntimeEvent::EthereumOutboundQueue(pezsnowbridge_pezpallet_outbound_queue::Event::MessageQueued {..}) => {},
			]
		);
	});
}

#[test]
fn register_weth_token_in_asset_hub_fail_for_insufficient_fee() {
	BridgeHubZagros::fund_para_sovereign(AssetHubZagros::para_id().into(), INITIAL_FUND);

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		type EthereumInboundQueue =
			<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumInboundQueue;
		let message_id: H256 = [0; 32].into();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::RegisterToken {
				token: WETH.into(),
				// Insufficient fee which should trigger the trap
				fee: INSUFFICIENT_XCM_FEE,
			},
		});
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id().into()).unwrap();

		assert_expected_events!(
			BridgeHubZagros,
			vec![
				RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success:false, .. }) => {},
			]
		);
	});
}

fn send_weth_from_ethereum_to_asset_hub_with_fee(account_id: [u8; 32], fee: u128) {
	// Fund asset hub sovereign on bridge hub
	let asset_hub_sovereign = BridgeHubZagros::sovereign_account_id_of(Location::new(
		1,
		[Teyrchain(AssetHubZagros::para_id().into())],
	));
	BridgeHubZagros::fund_accounts(vec![(asset_hub_sovereign.clone(), INITIAL_FUND)]);

	// Send WETH to an existent account on asset hub
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		type EthereumInboundQueue =
			<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumInboundQueue;
		let message_id: H256 = [0; 32].into();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::AccountId32 { id: account_id },
				amount: TOKEN_AMOUNT,
				fee,
			},
		});
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		assert_ok!(EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id().into()));

		// Check that the message was sent
		assert_expected_events!(
			BridgeHubZagros,
			vec![
				RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});
}

#[test]
fn send_weth_from_ethereum_to_existent_account_on_asset_hub() {
	send_weth_from_ethereum_to_asset_hub_with_fee(AssetHubZagrosSender::get().into(), XCM_FEE);

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},
			]
		);
	});
}

#[test]
fn send_weth_from_ethereum_to_non_existent_account_on_asset_hub() {
	send_weth_from_ethereum_to_asset_hub_with_fee([1; 32], XCM_FEE);

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},
			]
		);
	});
}

#[test]
fn send_weth_from_ethereum_to_non_existent_account_on_asset_hub_with_insufficient_fee() {
	send_weth_from_ethereum_to_asset_hub_with_fee([1; 32], INSUFFICIENT_XCM_FEE);

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		// Check that the message was not processed successfully due to insufficient fee

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success:false, .. }) => {},
			]
		);
	});
}

/// A bridged token is a sufficient asset, so receiving it opens the beneficiary's account on its
/// own — the native remainder left over from the fee has no say in it.
///
/// This replaces a test that asserted the opposite: that a remainder below the existential
/// deposit makes the whole message fail. That premise comes from a configuration where the
/// bridged token is not sufficient. Here it is, which is the same property that lets an account
/// hold a stablecoin on this chain without holding the native token, so the deposit path the old
/// test named is never reached by this route.
///
/// Measured while establishing that: the Asset Hub leg charges 6_323_242 and the existential
/// deposit is 3_333_333, a thirtieth of the relay's. Sending 8_000_000 leaves 1_676_758 — half an
/// existential deposit, deliberately — and the account is still created, by the token rather than
/// by the balance.
#[test]
fn send_weth_from_ethereum_opens_the_account_by_the_token_not_the_remainder() {
	let beneficiary: [u8; 32] = [1; 32];
	send_weth_from_ethereum_to_asset_hub_with_fee(beneficiary, 8_000_000);

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { who, .. }) => {
					who: *who == beneficiary.into(),
				},
				// The remainder cannot land anywhere, so it is trapped rather than delivered, and
				// the program reports that. This is not the same as the transfer failing: the
				// instruction that carries the token ran first and to completion, which is what the
				// split in `convert_send_token` is for. Measured, twice: a sub-existential-deposit
				// native amount is refused even by an account a sufficient asset has just opened,
				// and the bridge's own sovereign account on this chain cannot take it either.
				//
				// Asserting `success: true` here would mean choosing the one arrangement that
				// produces it — sending every remainder to the bridge instead of the beneficiary —
				// and that costs the refund of *large* unspent fees to buy a tidy flag. Trapped
				// dust is claimable; an unrefundable fee is not.
				RuntimeEvent::PezkuwiXcm(pezpallet_xcm::Event::AssetsTrapped { .. }) => {},
			]
		);

		// The remainder was under the existential deposit, so it never landed as a balance. The
		// account exists all the same.
		assert_eq!(
			<<AssetHubZagros as AssetHubZagrosPallet>::Balances as pezframe_support::traits::fungible::Inspect<_>>::balance(
				&beneficiary.into()
			),
			0
		);
	});
}

/// Tests the registering of a token as an asset on AssetHub, and then subsequently sending
/// a token from Ethereum to AssetHub.
#[test]
fn send_token_from_ethereum_to_asset_hub() {
	let asset_hub_sovereign = BridgeHubZagros::sovereign_account_id_of(Location::new(
		1,
		[Teyrchain(AssetHubZagros::para_id().into())],
	));
	// Fund AssetHub sovereign account so it can pay execution fees for the asset transfer
	BridgeHubZagros::fund_accounts(vec![(asset_hub_sovereign.clone(), INITIAL_FUND)]);

	// Fund ethereum sovereign on AssetHub
	AssetHubZagros::fund_accounts(vec![(AssetHubZagrosReceiver::get(), INITIAL_FUND)]);

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::AccountId32 { id: AssetHubZagrosReceiver::get().into() },
				amount: TOKEN_AMOUNT,
				fee: XCM_FEE,
			},
		});
		let (xcm, _) = EthereumInboundQueue::do_convert([0; 32].into(), message).unwrap();
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id().into()).unwrap();

		// Check that the message was sent
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},]
		);
	});
}

/// Tests the full cycle of token transfers:
/// - registering a token on AssetHub
/// - sending a token to AssetHub
/// - returning the token to Ethereum
#[test]
fn send_weth_asset_from_asset_hub_to_ethereum() {
	let assethub_location = BridgeHubZagros::sibling_location_of(AssetHubZagros::para_id());
	let assethub_sovereign = BridgeHubZagros::sovereign_account_id_of(assethub_location);

	BridgeHubZagros::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::AccountId32 { id: AssetHubZagrosReceiver::get().into() },
				amount: TOKEN_AMOUNT,
				fee: XCM_FEE,
			},
		});
		let (xcm, _) = EthereumInboundQueue::do_convert([0; 32].into(), message).unwrap();
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id().into()).unwrap();

		// Check that the send token message was sent using xcm
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) =>{},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;

		// Check that AssetHub has issued the foreign asset
		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},]
		);
		let assets = vec![Asset {
			id: AssetId(Location::new(
				2,
				[
					GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID }),
					AccountKey20 { network: None, key: WETH },
				],
			)),
			fun: Fungible(TOKEN_AMOUNT),
		}];
		let versioned_assets = VersionedAssets::from(Assets::from(assets.clone()));

		let destination = VersionedLocation::from(Location::new(
			2,
			[GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })],
		));

		let beneficiary = VersionedLocation::from(Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		));

		let free_balance_before = <AssetHubZagros as AssetHubZagrosPallet>::Balances::free_balance(
			AssetHubZagrosReceiver::get(),
		);
		// Send the Weth back to Ethereum
		let fee_asset_id: AssetId = AssetId(Location::new(
			2,
			[
				GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID }),
				AccountKey20 { network: None, key: WETH },
			],
		));
		<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::limited_reserve_transfer_assets(
			RuntimeOrigin::signed(AssetHubZagrosReceiver::get()),
			Box::new(destination),
			Box::new(beneficiary),
			Box::new(versioned_assets),
			fee_asset_index(&Assets::from(assets.clone()), &fee_asset_id),
			Unlimited,
		)
		.unwrap();
		let free_balance_after = <AssetHubZagros as AssetHubZagrosPallet>::Balances::free_balance(
			AssetHubZagrosReceiver::get(),
		);
		// Assert at least DefaultBridgeHubEthereumBaseFee charged from the sender
		let free_balance_diff = free_balance_before - free_balance_after;
		assert!(free_balance_diff > DefaultBridgeHubEthereumBaseFee::get());
	});

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumOutboundQueue(pezsnowbridge_pezpallet_outbound_queue::Event::MessageQueued{ .. }) => {},]
		);
	});
}

/// Tests sending a token to a 3rd party teyrchain, called PenPal. The token reserve is
/// still located on AssetHub.
#[test]
fn send_token_from_ethereum_to_penpal() {
	let asset_hub_sovereign = BridgeHubZagros::sovereign_account_id_of(Location::new(
		1,
		[Teyrchain(AssetHubZagros::para_id().into())],
	));
	// Fund AssetHub sovereign account so it can pay execution fees for the asset transfer
	BridgeHubZagros::fund_accounts(vec![(asset_hub_sovereign.clone(), INITIAL_FUND)]);
	// Fund PenPal receiver (covering ED)
	PenpalB::fund_accounts(vec![(PenpalBReceiver::get(), INITIAL_FUND)]);

	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })]).encode(),
			)],
		));
	});

	let ethereum_network_v5: NetworkId = EthereumNetwork::get().into();

	// The Weth asset location, identified by the contract address on Ethereum
	let weth_asset_location: Location =
		(Parent, Parent, ethereum_network_v5, AccountKey20 { network: None, key: WETH }).into();

	// Fund ethereum sovereign on AssetHub
	let ethereum_sovereign: AccountId = pezsnowbridge_sovereign();
	AssetHubZagros::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);

	// Create asset on the Penpal teyrchain.
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::force_create(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			weth_asset_location.clone(),
			asset_hub_sovereign.clone().into(),
			false,
			1000,
		));

		assert!(<PenpalB as PenpalBPallet>::ForeignAssets::asset_exists(
			weth_asset_location.clone()
		));
	});

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::ForeignAccountId32 {
					para_id: PARA_ID_B,
					id: PenpalBReceiver::get().into(),
					fee: 100_000_000_000u128,
				},
				amount: TOKEN_AMOUNT,
				fee: XCM_FEE,
			},
		});
		let (xcm, _) = EthereumInboundQueue::do_convert([0; 32].into(), message).unwrap();
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id().into()).unwrap();

		// Check that the send token message was sent using xcm
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) =>{},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		// Check that the assets were issued on AssetHub
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},
				RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	PenpalB::execute_with(|| {
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;
		// Check that the assets were issued on PenPal
		assert_expected_events!(
			PenpalB,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},
			]
		);
	});
}

#[test]
fn transfer_relay_token() {
	let assethub_sovereign = BridgeHubZagros::sovereign_account_id_of(
		BridgeHubZagros::sibling_location_of(AssetHubZagros::para_id()),
	);
	BridgeHubZagros::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);

	let asset_id: Location = Location { parents: 1, interior: [].into() };
	let expected_asset_id: Location =
		Location { parents: 1, interior: [GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH))].into() };

	let expected_token_id = TokenIdOf::convert_location(&expected_asset_id).unwrap();

	let ethereum_sovereign: AccountId = pezsnowbridge_sovereign();

	// Register token
	BridgeHubZagros::execute_with(|| {
		type RuntimeOrigin = <BridgeHubZagros as Chain>::RuntimeOrigin;
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		assert_ok!(<BridgeHubZagros as BridgeHubZagrosPallet>::Balances::force_set_balance(
			RuntimeOrigin::root(),
			pezsp_runtime::MultiAddress::Id(BridgeHubZagrosSender::get()),
			INITIAL_FUND * 10,
		));

		assert_ok!(<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::from(asset_id.clone())),
			AssetMetadata {
				name: "wnd".as_bytes().to_vec().try_into().unwrap(),
				symbol: "wnd".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
		// Check that a message was sent to Ethereum to create the agent
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumSystem(pezsnowbridge_pezpallet_system::Event::RegisterToken { .. }) => {},]
		);
	});

	// Send token to Ethereum
	AssetHubZagros::execute_with(|| {
		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		let assets = vec![Asset { id: AssetId(Location::parent()), fun: Fungible(TOKEN_AMOUNT) }];
		let versioned_assets = VersionedAssets::from(Assets::from(assets.clone()));

		let destination = VersionedLocation::from(Location::new(
			2,
			[GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })],
		));

		let beneficiary = Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		);

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
			RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			Box::new(destination),
			Box::new(versioned_assets),
			Box::new(TransferType::LocalReserve),
			Box::new(VersionedAssetId::from(AssetId(Location::parent()))),
			Box::new(TransferType::LocalReserve),
			Box::new(VersionedXcm::from(
				Xcm::<()>::builder_unsafe()
					.deposit_asset(AllCounted(1), beneficiary)
					.build()
			)),
			Unlimited,
		));

		let events = AssetHubZagros::events();
		// Check that the native asset transferred to some reserved account(sovereign of Ethereum)
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Balances(pezpallet_balances::Event::Transfer { amount, to, ..})
					if *amount == TOKEN_AMOUNT && *to == ethereum_sovereign.clone(),
			)),
			"native token reserved to Ethereum sovereign account."
		);
	});

	// Send token back from ethereum
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumOutboundQueue(pezsnowbridge_pezpallet_outbound_queue::Event::MessageQueued{ .. }) => {},]
		);

		// Send relay token back to AH
		let message_id: H256 = [0; 32].into();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendNativeToken {
				token_id: expected_token_id,
				destination: Destination::AccountId32 { id: AssetHubZagrosReceiver::get().into() },
				amount: TOKEN_AMOUNT,
				fee: XCM_FEE,
			},
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id().into()).unwrap();

		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::Balances(pezpallet_balances::Event::Withdraw{ .. }) => {},]
		);

		let events = AssetHubZagros::events();

		// Check that the native token burnt from some reserved account
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Balances(pezpallet_balances::Event::Withdraw { who, ..})
					if *who == ethereum_sovereign.clone(),
			)),
			"native token burnt from Ethereum sovereign account."
		);

		// Check that the token was minted to beneficiary
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Balances(pezpallet_balances::Event::Deposit { who, amount })
					if *amount >= TOKEN_AMOUNT && *who == AssetHubZagrosReceiver::get()
			)),
			"Token minted to beneficiary."
		);
	});
}

#[test]
fn transfer_ah_token() {
	let assethub_sovereign = BridgeHubZagros::sovereign_account_id_of(
		BridgeHubZagros::sibling_location_of(AssetHubZagros::para_id()),
	);
	BridgeHubZagros::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);

	let ethereum_destination =
		Location::new(2, [GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })]);

	let ethereum_sovereign: AccountId = pezsnowbridge_sovereign();
	AssetHubZagros::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);

	let asset_id: Location =
		[PalletInstance(ASSETS_PALLET_ID), GeneralIndex(RESERVABLE_ASSET_ID.into())].into();

	let asset_id_in_bh: Location = Location::new(
		1,
		[
			Teyrchain(AssetHubZagros::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(RESERVABLE_ASSET_ID.into()),
		],
	);

	let asset_id_after_reanchored = Location::new(
		1,
		[
			GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
			Teyrchain(AssetHubZagros::para_id().into()),
		],
	)
	.appended_with(asset_id.clone().interior)
	.unwrap();

	let token_id = TokenIdOf::convert_location(&asset_id_after_reanchored).unwrap();

	// Register token
	BridgeHubZagros::execute_with(|| {
		type RuntimeOrigin = <BridgeHubZagros as Chain>::RuntimeOrigin;

		assert_ok!(<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::from(asset_id_in_bh.clone())),
			AssetMetadata {
				name: "ah_asset".as_bytes().to_vec().try_into().unwrap(),
				symbol: "ah_asset".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
	});

	// Mint some token
	AssetHubZagros::mint_asset(
		<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosAssetOwner::get()),
		RESERVABLE_ASSET_ID,
		AssetHubZagrosSender::get(),
		TOKEN_AMOUNT,
	);

	// Send token to Ethereum
	AssetHubZagros::execute_with(|| {
		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		// Send partial of the token, will fail if send all
		let assets =
			vec![Asset { id: AssetId(asset_id.clone()), fun: Fungible(TOKEN_AMOUNT / 10) }];
		let versioned_assets = VersionedAssets::from(Assets::from(assets.clone()));

		let beneficiary = VersionedLocation::from(Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		));

		let fee_asset_id: AssetId = AssetId(asset_id.clone());
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::transfer_assets(
			RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			Box::new(VersionedLocation::from(ethereum_destination)),
			Box::new(beneficiary),
			Box::new(versioned_assets),
			fee_asset_index(&Assets::from(assets.clone()), &fee_asset_id),
			Unlimited,
		));

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::Assets(pezpallet_assets::Event::Transferred{ .. }) => {},]
		);

		let events = AssetHubZagros::events();
		// Check that the native asset transferred to some reserved account(sovereign of Ethereum)
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Assets(pezpallet_assets::Event::Transferred { asset_id, to, ..})
					if *asset_id == RESERVABLE_ASSET_ID && *to == ethereum_sovereign.clone()
			)),
			"native token reserved to Ethereum sovereign account."
		);
	});

	// Send token back from Ethereum
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumOutboundQueue(pezsnowbridge_pezpallet_outbound_queue::Event::MessageQueued{ .. }) => {},]
		);

		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendNativeToken {
				token_id,
				destination: Destination::AccountId32 { id: AssetHubZagrosReceiver::get().into() },
				amount: TOKEN_AMOUNT / 10,
				fee: XCM_FEE,
			},
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert([0; 32].into(), message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id().into()).unwrap();

		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::Assets(pezpallet_assets::Event::Withdrawn{..}) => {},]
		);

		let events = AssetHubZagros::events();

		// Check that the native token burnt from some reserved account
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Assets(pezpallet_assets::Event::Withdrawn { who, .. })
					if *who == ethereum_sovereign.clone(),
			)),
			"token burnt from Ethereum sovereign account."
		);

		// Check that the token was minted to beneficiary
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Assets(pezpallet_assets::Event::Deposited { who, .. })
					if *who == AssetHubZagrosReceiver::get()
			)),
			"Token minted to beneficiary."
		);
	});
}

// Tests a full cycle of transferring weth from Ethereum to AHW, to AHR using the P<>K bridge,
// and back again to Ethereum. The transaction is done in 2 transaction hops, per direction.
#[test]
fn send_weth_from_ethereum_to_ahw_to_ahr_back_to_ahw_and_ethereum() {
	let sender = AssetHubZagrosSender::get();
	BridgeHubZagros::fund_para_sovereign(AssetHubZagros::para_id(), INITIAL_FUND);
	BridgeHubPezkuwichain::fund_para_sovereign(AssetHubPezkuwichain::para_id(), INITIAL_FUND);
	let ethereum_destination =
		Location::new(2, [GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })]);
	let ethereum_sovereign: AccountId = AssetHubZagros::execute_with(|| {
		ExternalConsensusLocationsConverterFor::<
			AssetHubZagrosUniversalLocation,
			[u8; 32],
		>::convert_location(&ethereum_destination.clone())
			.unwrap()
			.into()
	});
	AssetHubZagros::fund_accounts(vec![
		(ethereum_sovereign, INITIAL_FUND),
		// to pay fees to AHR
		(sender.clone(), INITIAL_FUND),
	]);

	let bridged_wnd_at_asset_hub_pezkuwichain = bridged_wnd_at_ah_pezkuwichain();
	let wnd_reserve = vec![(asset_hub_zagros_global_location(), false).into()];
	create_foreign_on_ah_pezkuwichain(
		bridged_wnd_at_asset_hub_pezkuwichain.clone(),
		true,
		wnd_reserve,
	);
	create_foreign_pool_with_parent_native_on!(
		AssetHubPezkuwichain,
		bridged_wnd_at_asset_hub_pezkuwichain.clone(),
		AssetHubPezkuwichainSender::get()
	);

	// set XCM versions
	BridgeHubZagros::force_xcm_version(asset_hub_zagros_global_location(), XCM_VERSION);
	BridgeHubZagros::force_xcm_version(asset_hub_pezkuwichain_location(), XCM_VERSION);
	AssetHubZagros::force_xcm_version(asset_hub_pezkuwichain_location(), XCM_VERSION);
	AssetHubPezkuwichain::force_xcm_version(asset_hub_zagros_global_location(), XCM_VERSION);
	BridgeHubPezkuwichain::force_xcm_version(asset_hub_zagros_global_location(), XCM_VERSION);
	BridgeHubPezkuwichain::force_xcm_version(asset_hub_pezkuwichain_location(), XCM_VERSION);

	// Bridge token from Ethereum to AHP
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		// Construct SendToken message and sent to inbound queue
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendToken {
				token: WETH.into(),
				destination: Destination::AccountId32 { id: sender.clone().into() },
				amount: TOKEN_AMOUNT,
				fee: XCM_FEE,
			},
		});
		// Convert the message to XCM
		let message_id: H256 = [1; 32].into();
		let (xcm, _) = EthereumInboundQueue::do_convert(message_id, message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id()).unwrap();

		// Check that the message was sent
		assert_expected_events!(
			BridgeHubZagros,
			vec![
				RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},
			]
		);
	});

	let beneficiary = Location::new(
		0,
		[AccountId32 { network: None, id: AssetHubPezkuwichainReceiver::get().into() }],
	);
	let weth_location = Location::new(
		2,
		[GlobalConsensus(EthereumNetwork::get()), AccountKey20 { network: None, key: WETH }],
	);
	let fee: Location = Parent.into(); // Hez
	let fees_asset: AssetId = fee.clone().into();
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(2)),
		beneficiary: beneficiary.clone(),
	}]);

	let assets: Assets =
		vec![(weth_location.clone(), TOKEN_AMOUNT).into(), (fee, XCM_FEE * 3).into()].into();

	assert_ok!(AssetHubZagros::execute_with(|| {
		<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(sender.clone()),
			bx!(asset_hub_pezkuwichain_location().into()),
			bx!(assets.clone().into()),
			bx!(TransferType::LocalReserve),
			bx!(fees_asset.clone().into()),
			bx!(TransferType::LocalReserve),
			bx!(VersionedXcm::from(custom_xcm_on_dest.clone())),
			WeightLimit::Unlimited,
		)
	}));

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		let events = AssetHubZagros::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PezkuwiXcm(pezpallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped, should not happen."
		);
	});

	assert_bridge_hub_zagros_message_accepted(true);
	assert_bridge_hub_pezkuwichain_message_received();

	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				// Token was issued to beneficiary
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { asset_id, who, .. }) => {
					asset_id: *asset_id == weth_location,
					who: *who == AssetHubPezkuwichainReceiver::get().into(),
				},
			]
		);

		let events = AssetHubPezkuwichain::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PezkuwiXcm(pezpallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped, should not happen."
		);
	});

	let beneficiary =
		Location::new(0, [AccountId32 { network: None, id: AssetHubZagrosReceiver::get().into() }]);
	let fee = bridged_wnd_at_asset_hub_pezkuwichain;
	let fees_asset: AssetId = fee.clone().into();
	let custom_xcm_on_dest =
		Xcm::<()>(vec![DepositAsset { assets: Wild(AllCounted(2)), beneficiary }]);

	let assets: Assets =
		vec![(weth_location.clone(), TOKEN_AMOUNT).into(), (fee, XCM_FEE).into()].into();

	// Transfer the token back to Zagros.
	assert_ok!(AssetHubPezkuwichain::execute_with(|| {
		<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(AssetHubPezkuwichainReceiver::get()),
			bx!(asset_hub_zagros_global_location().into()),
			bx!(assets.into()),
			bx!(TransferType::DestinationReserve),
			bx!(fees_asset.into()),
			bx!(TransferType::DestinationReserve),
			bx!(VersionedXcm::from(custom_xcm_on_dest)),
			WeightLimit::Unlimited,
		)
	}));

	BridgeHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <BridgeHubPezkuwichain as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubPezkuwichain,
			vec![
				// message exported
				RuntimeEvent::BridgeZagrosMessages(
					pezpallet_bridge_messages::Event::MessageAccepted { .. }
				) => {},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubZagros,
			vec![
				// message sent to destination
				RuntimeEvent::XcmpQueue(
					pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
			]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		// Check that the token was received and issued as a foreign asset on AssetHub
		assert_expected_events!(
			AssetHubZagros,
			vec![
				// Token was issued to beneficiary
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { asset_id, who, .. }) => {
					asset_id: *asset_id == weth_location,
					who: *who == AssetHubZagrosReceiver::get().into(),
				},
			]
		);

		let events = AssetHubZagros::events();
		// Check that no assets were trapped
		assert!(
			!events.iter().any(|event| matches!(
				event,
				RuntimeEvent::PezkuwiXcm(pezpallet_xcm::Event::AssetsTrapped { .. })
			)),
			"Assets were trapped, should not happen."
		);
	});

	// Transfer the token back to Ethereum.
	AssetHubZagros::execute_with(|| {
		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;

		let assets = vec![Asset {
			id: AssetId(Location::new(
				2,
				[
					GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID }),
					AccountKey20 { network: None, key: WETH },
				],
			)),
			fun: Fungible(TOKEN_AMOUNT),
		}];

		let versioned_assets = VersionedAssets::from(Assets::from(assets.clone()));

		let destination = VersionedLocation::from(Location::new(
			2,
			[GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })],
		));

		let beneficiary = VersionedLocation::from(Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		));

		let free_balance_before = <AssetHubZagros as AssetHubZagrosPallet>::Balances::free_balance(
			AssetHubZagrosReceiver::get(),
		);
		// Send the Weth back to Ethereum
		let fee_asset_id: AssetId = AssetId(Location::new(
			2,
			[
				GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID }),
				AccountKey20 { network: None, key: WETH },
			],
		));
		<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::limited_reserve_transfer_assets(
			RuntimeOrigin::signed(AssetHubZagrosReceiver::get()),
			Box::new(destination),
			Box::new(beneficiary),
			Box::new(versioned_assets),
			fee_asset_index(&Assets::from(assets.clone()), &fee_asset_id),
			Unlimited,
		)
		.unwrap();
		let free_balance_after = <AssetHubZagros as AssetHubZagrosPallet>::Balances::free_balance(
			AssetHubZagrosReceiver::get(),
		);
		// Assert at least DefaultBridgeHubEthereumBaseFee charged from the sender
		let free_balance_diff = free_balance_before - free_balance_after;
		assert!(free_balance_diff > DefaultBridgeHubEthereumBaseFee::get());
	});

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumOutboundQueue(pezsnowbridge_pezpallet_outbound_queue::Event::MessageQueued{ .. }) => {},]
		);
	});
}

#[test]
fn transfer_penpal_native_asset() {
	let assethub_sovereign = BridgeHubZagros::sovereign_account_id_of(
		BridgeHubZagros::sibling_location_of(AssetHubZagros::para_id()),
	);
	BridgeHubZagros::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);

	let pal_at_asset_hub = Location::new(1, [Teyrchain(PenpalB::para_id().into())]);

	let pal_after_reanchored = Location::new(
		1,
		[GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)), Teyrchain(PenpalB::para_id().into())],
	);

	let token_id = TokenIdOf::convert_location(&pal_after_reanchored).unwrap();

	let asset_owner = PenpalAssetOwner::get();

	AssetHubZagros::force_create_foreign_asset(
		pal_at_asset_hub.clone(),
		asset_owner.clone().into(),
		true,
		1,
		vec![],
	);
	// Set "pal" as teleportable between Penpal and AH, using the asset owner account
	AssetHubZagros::set_foreign_asset_reserves(
		pal_at_asset_hub.clone(),
		asset_owner.into(),
		vec![(pal_at_asset_hub.clone(), true).into()],
	);

	let penpal_sovereign = AssetHubZagros::sovereign_account_id_of(
		AssetHubZagros::sibling_location_of(PenpalB::para_id()),
	);
	AssetHubZagros::fund_accounts(vec![(penpal_sovereign.clone(), INITIAL_FUND)]);

	// Register token
	BridgeHubZagros::execute_with(|| {
		type RuntimeOrigin = <BridgeHubZagros as Chain>::RuntimeOrigin;

		assert_ok!(<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::from(pal_at_asset_hub.clone())),
			AssetMetadata {
				name: "pal".as_bytes().to_vec().try_into().unwrap(),
				symbol: "pal".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
	});

	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			Location::parent(),
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
	});

	// Send PAL to Ethereum
	PenpalB::execute_with(|| {
		type RuntimeOrigin = <PenpalB as Chain>::RuntimeOrigin;
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

		// HEZ as fee
		let assets = vec![
			// Should cover the bridge fee
			Asset { id: AssetId(Location::parent()), fun: Fungible(BRIDGE_FEE) },
			Asset { id: AssetId(Location::here()), fun: Fungible(TOKEN_AMOUNT) },
		];

		let beneficiary = Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		);

		let destination = Location::new(1, [Teyrchain(AssetHubZagros::para_id().into())]);

		let custom_xcm_on_dest = Xcm::<()>(vec![DepositReserveAsset {
			assets: Wild(AllOf {
				id: AssetId(pal_at_asset_hub.clone()),
				fun: WildFungibility::Fungible,
			}),
			dest: ethereum(),
			xcm: vec![
				BuyExecution {
					fees: Asset {
						id: AssetId(pal_after_reanchored.clone()),
						fun: Fungible(TOKEN_AMOUNT),
					},
					weight_limit: Unlimited,
				},
				DepositAsset { assets: Wild(AllCounted(1)), beneficiary },
			]
			.into(),
		}]);

		assert_ok!(<PenpalB as PenpalBPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
			RuntimeOrigin::signed(PenpalBSender::get()),
			Box::new(VersionedLocation::from(destination)),
			Box::new(VersionedAssets::from(assets.clone())),
			Box::new(TransferType::Teleport),
			Box::new(VersionedAssetId::from(AssetId(Location::parent()))),
			Box::new(TransferType::DestinationReserve),
			Box::new(VersionedXcm::from(custom_xcm_on_dest)),
			Unlimited,
		));

		assert_expected_events!(
			PenpalB,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Withdrawn{ .. }) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},]
		);
	});

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumOutboundQueue(pezsnowbridge_pezpallet_outbound_queue::Event::MessageQueued{ .. }) => {},]
		);
	});

	// Send PAL back from Ethereum
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendNativeToken {
				token_id,
				destination: Destination::AccountId32 { id: AssetHubZagrosSender::get().into() },
				amount: TOKEN_AMOUNT,
				fee: XCM_FEE,
			},
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert([0; 32].into(), message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id()).unwrap();

		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Withdrawn{..}) => {},]
		);

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited{..}) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;

		let destination = AssetHubZagros::sibling_location_of(PenpalB::para_id());

		let beneficiary =
			Location::new(0, [AccountId32 { network: None, id: PenpalBReceiver::get().into() }]);

		// HEZ as fee
		let assets =
			vec![Asset { id: AssetId(pal_at_asset_hub.clone()), fun: Fungible(TOKEN_AMOUNT) }];

		let fee_asset_id: AssetId = AssetId(pal_at_asset_hub.clone());
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::limited_teleport_assets(
			RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			Box::new(VersionedLocation::from(destination)),
			Box::new(VersionedLocation::from(beneficiary)),
			Box::new(VersionedAssets::from(assets.clone())),
			fee_asset_index(&Assets::from(assets.clone()), &fee_asset_id),
			Unlimited,
		));

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Withdrawn{..}) => {},]
		);
	});

	PenpalB::execute_with(|| {
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

		assert_expected_events!(
			PenpalB,
			vec![RuntimeEvent::Balances(pezpallet_balances::Event::Deposit{..}) => {},]
		);
	})
}

#[test]
fn transfer_penpal_teleport_enabled_asset() {
	let assethub_sovereign = BridgeHubZagros::sovereign_account_id_of(
		BridgeHubZagros::sibling_location_of(AssetHubZagros::para_id()),
	);
	BridgeHubZagros::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);

	let asset_location_on_penpal =
		PenpalB::execute_with(|| PenpalLocalTeleportableToAssetHub::get());

	let pal_at_asset_hub = Location::new(1, [Junction::Teyrchain(PenpalB::para_id().into())])
		.appended_with(asset_location_on_penpal.clone())
		.unwrap();

	let pal_after_reanchored = Location::new(
		1,
		[GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)), Teyrchain(PenpalB::para_id().into())],
	)
	.appended_with(asset_location_on_penpal.clone())
	.unwrap();

	let token_id = TokenIdOf::convert_location(&pal_after_reanchored).unwrap();

	let penpal_sovereign = AssetHubZagros::sovereign_account_id_of(
		AssetHubZagros::sibling_location_of(PenpalB::para_id()),
	);
	AssetHubZagros::fund_accounts(vec![(penpal_sovereign.clone(), INITIAL_FUND)]);
	AssetHubZagros::fund_accounts(vec![(pezsnowbridge_sovereign(), INITIAL_FUND)]);

	// Register token
	BridgeHubZagros::execute_with(|| {
		type RuntimeOrigin = <BridgeHubZagros as Chain>::RuntimeOrigin;

		assert_ok!(<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::from(pal_at_asset_hub.clone())),
			AssetMetadata {
				name: "pal".as_bytes().to_vec().try_into().unwrap(),
				symbol: "pal".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
	});

	// Fund on Penpal
	PenpalB::fund_accounts(vec![(CheckingAccount::get(), INITIAL_FUND)]);
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::Assets::mint_into(
			PEN2_TELEPORTABLE_ASSET_ID,
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			Location::parent(),
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
	});

	// Send PAL to Ethereum
	PenpalB::execute_with(|| {
		type RuntimeOrigin = <PenpalB as Chain>::RuntimeOrigin;
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

		// HEZ as fee
		let assets = vec![
			// Should cover the bridge fee
			Asset { id: AssetId(Location::parent()), fun: Fungible(BRIDGE_FEE) },
			Asset { id: AssetId(asset_location_on_penpal.clone()), fun: Fungible(TOKEN_AMOUNT) },
		];

		let beneficiary = Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		);

		let destination = Location::new(1, [Teyrchain(AssetHubZagros::para_id().into())]);

		let custom_xcm_on_dest = Xcm::<()>(vec![DepositReserveAsset {
			assets: Wild(AllOf {
				id: AssetId(pal_at_asset_hub.clone()),
				fun: WildFungibility::Fungible,
			}),
			dest: ethereum(),
			xcm: vec![
				BuyExecution {
					fees: Asset {
						id: AssetId(pal_after_reanchored.clone()),
						fun: Fungible(TOKEN_AMOUNT),
					},
					weight_limit: Unlimited,
				},
				DepositAsset { assets: Wild(AllCounted(1)), beneficiary },
			]
			.into(),
		}]);

		assert_ok!(<PenpalB as PenpalBPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
			RuntimeOrigin::signed(PenpalBSender::get()),
			Box::new(VersionedLocation::from(destination)),
			Box::new(VersionedAssets::from(assets.clone())),
			Box::new(TransferType::Teleport),
			Box::new(VersionedAssetId::from(AssetId(Location::parent()))),
			Box::new(TransferType::DestinationReserve),
			Box::new(VersionedXcm::from(custom_xcm_on_dest)),
			Unlimited,
		));

		assert_expected_events!(
			PenpalB,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Withdrawn{ .. }) => {},]
		);

		assert_expected_events!(
			PenpalB,
			vec![RuntimeEvent::Assets(pezpallet_assets::Event::Withdrawn{ .. }) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { .. }) => {},]
		);
	});

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumOutboundQueue(pezsnowbridge_pezpallet_outbound_queue::Event::MessageQueued{ .. }) => {},]
		);
	});

	// Send PAL back from Ethereum
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendNativeToken {
				token_id,
				destination: Destination::AccountId32 { id: AssetHubZagrosSender::get().into() },
				amount: TOKEN_AMOUNT,
				fee: XCM_FEE,
			},
		});
		// Convert the message to XCM
		let (xcm, _) = EthereumInboundQueue::do_convert([0; 32].into(), message).unwrap();
		// Send the XCM
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id()).unwrap();

		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) =>
	{},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Withdrawn{..}) => {},]
		);

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited{..}) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;

		let destination = AssetHubZagros::sibling_location_of(PenpalB::para_id());

		let beneficiary =
			Location::new(0, [AccountId32 { network: None, id: PenpalBReceiver::get().into() }]);

		// HEZ as fee
		let assets = vec![
			Asset { id: AssetId(Location::parent()), fun: Fungible(XCM_FEE) },
			Asset { id: AssetId(pal_at_asset_hub.clone()), fun: Fungible(TOKEN_AMOUNT) },
		];

		let custom_xcm_on_dest = Xcm::<()>(vec![
			BuyExecution {
				fees: Asset { id: AssetId(Location::parent()), fun: Fungible(XCM_FEE) },
				weight_limit: Unlimited,
			},
			DepositAsset { assets: Wild(AllCounted(2)), beneficiary },
		]);

		assert_ok!(
			<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
				RuntimeOrigin::signed(AssetHubZagrosSender::get()),
				Box::new(VersionedLocation::from(destination)),
				Box::new(VersionedAssets::from(assets.clone())),
				Box::new(TransferType::Teleport),
				Box::new(VersionedAssetId::from(AssetId(Location::parent()))),
				Box::new(TransferType::LocalReserve),
				Box::new(VersionedXcm::from(custom_xcm_on_dest)),
				Unlimited,
			)
		);

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Withdrawn{..}) => {},]
		);
	});

	PenpalB::execute_with(|| {
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;

		assert_expected_events!(
			PenpalB,
			vec![RuntimeEvent::Assets(pezpallet_assets::Event::Deposited{..}) => {},]
		);
	})
}

#[test]
fn mint_native_asset_on_penpal_from_relay_chain() {
	// Send XCM message from Relay Chain to Penpal
	Zagros::execute_with(|| {
		Dmp::make_teyrchain_reachable(PenpalB::para_id());
		// Set balance call
		let mint_token_call = hex!("0a0800d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d0f00406352bfc601");
		let remote_xcm = VersionedXcm::from(Xcm(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			Transact {
				origin_kind: OriginKind::Superuser,
				fallback_max_weight: None,
				call: mint_token_call.to_vec().into(),
			},
		]));
		assert_ok!(<Zagros as ZagrosPallet>::XcmPallet::send(
			<Zagros as Chain>::RuntimeOrigin::root(),
			bx!(VersionedLocation::from(Location::new(0, [Teyrchain(PenpalB::para_id().into())]))),
			bx!(remote_xcm),
		));

		type RuntimeEvent = <Zagros as Chain>::RuntimeEvent;
		// Check that the Transact message was sent
		assert_expected_events!(
			Zagros,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PenpalB::execute_with(|| {
		type RuntimeEvent = <PenpalB as Chain>::RuntimeEvent;
		// Check that a message was sent to Ethereum to create the agent
		assert_expected_events!(
			PenpalB,
			vec![
				RuntimeEvent::Balances(pezpallet_balances::Event::BalanceSet {
					..
				}) => {},
			]
		);
	});
}

pub(crate) fn set_up_pool_with_wnd_on_ah_zagros(
	asset: Location,
	is_foreign: bool,
	initial_fund: u128,
	initial_liquidity: u128,
) {
	let wnd: Location = Parent.into();
	AssetHubZagros::fund_accounts(vec![(AssetHubZagrosSender::get(), initial_fund)]);
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		let owner = AssetHubZagrosSender::get();
		let signed_owner = <AssetHubZagros as Chain>::RuntimeOrigin::signed(owner.clone());

		if is_foreign {
			assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint(
				signed_owner.clone(),
				asset.clone().into(),
				owner.clone().into(),
				initial_fund,
			));
		} else {
			let asset_id = match asset.interior.last() {
				Some(GeneralIndex(id)) => *id as u32,
				_ => unreachable!(),
			};
			assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::Assets::mint(
				signed_owner.clone(),
				asset_id.into(),
				owner.clone().into(),
				initial_fund,
			));
		}
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::create_pool(
			signed_owner.clone(),
			Box::new(wnd.clone()),
			Box::new(asset.clone()),
		));
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::add_liquidity(
			signed_owner.clone(),
			Box::new(wnd),
			Box::new(asset),
			initial_liquidity,
			initial_liquidity,
			1,
			1,
			owner.into()
		));
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::LiquidityAdded {..}) => {},
			]
		);
	});
}

#[test]
fn transfer_roc_from_ah_with_legacy_api_will_fail() {
	let assethub_sovereign = BridgeHubZagros::sovereign_account_id_of(
		BridgeHubZagros::sibling_location_of(AssetHubZagros::para_id()),
	);
	BridgeHubZagros::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);

	let ethereum_destination =
		Location::new(2, [GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })]);

	let ethereum_sovereign: AccountId = pezsnowbridge_sovereign();
	AssetHubZagros::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);

	let bridged_roc_at_asset_hub_zagros = bridged_roc_at_ah_zagros();

	create_foreign_on_ah_zagros(
		bridged_roc_at_asset_hub_zagros.clone(),
		true,
		vec![(asset_hub_pezkuwichain_location(), false).into()],
		vec![],
	);

	let asset_id: Location = bridged_roc_at_asset_hub_zagros.clone();

	let initial_fund: u128 = 200_000_000_000_000;
	let initial_liquidity: u128 = initial_fund / 2;
	// Setup pool and add liquidity
	set_up_pool_with_wnd_on_ah_zagros(
		bridged_roc_at_asset_hub_zagros.clone(),
		true,
		initial_fund,
		initial_liquidity,
	);

	register_roc_on_bh();

	// Send token to Ethereum
	AssetHubZagros::execute_with(|| {
		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;

		// Send partial of the token, will fail if send all
		let assets =
			vec![Asset { id: AssetId(asset_id.clone()), fun: Fungible(initial_fund / 10) }];
		let versioned_assets = VersionedAssets::from(Assets::from(assets.clone()));

		let beneficiary = VersionedLocation::from(Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		));

		let fee_asset_id: AssetId = AssetId(asset_id.clone());
		let result = <AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::transfer_assets(
			RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			Box::new(VersionedLocation::from(ethereum_destination)),
			Box::new(beneficiary),
			Box::new(versioned_assets),
			fee_asset_index(&Assets::from(assets.clone()), &fee_asset_id),
			Unlimited,
		);

		assert_err!(
			result,
			DispatchError::Module(pezsp_runtime::ModuleError {
				index: 31,
				error: [21, 0, 0, 0],
				message: Some("InvalidAssetUnknownReserve")
			})
		);
	});
}

#[test]
fn transfer_roc_from_ah_with_transfer_and_then() {
	let assethub_sovereign = BridgeHubZagros::sovereign_account_id_of(
		BridgeHubZagros::sibling_location_of(AssetHubZagros::para_id()),
	);
	BridgeHubZagros::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);

	let ethereum_destination =
		Location::new(2, [GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })]);

	let ethereum_sovereign: AccountId = pezsnowbridge_sovereign();
	AssetHubZagros::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);

	let bridged_roc_at_asset_hub_zagros = bridged_roc_at_ah_zagros();

	create_foreign_on_ah_zagros(
		bridged_roc_at_asset_hub_zagros.clone(),
		true,
		vec![(asset_hub_pezkuwichain_location(), false).into()],
		vec![],
	);

	let asset_id: Location = bridged_roc_at_asset_hub_zagros.clone();

	let initial_fund: u128 = 200_000_000_000_000;
	let initial_liquidity: u128 = initial_fund / 2;
	// Setup pool and add liquidity
	set_up_pool_with_wnd_on_ah_zagros(
		bridged_roc_at_asset_hub_zagros.clone(),
		true,
		initial_fund,
		initial_liquidity,
	);

	register_roc_on_bh();

	// Send token to Ethereum
	AssetHubZagros::execute_with(|| {
		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		// Send partial of the token, will fail if send all
		let asset = Asset { id: AssetId(asset_id.clone()), fun: Fungible(initial_fund / 10) };
		let assets = vec![asset.clone()];
		let versioned_assets = VersionedAssets::from(Assets::from(assets.clone()));

		let beneficiary = Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		);

		let custom_xcm = Xcm::<()>(vec![DepositAsset {
			assets: Wild(AllCounted(assets.len() as u32)),
			beneficiary,
		}]);

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
			RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			Box::new(VersionedLocation::from(ethereum_destination)),
			Box::new(versioned_assets),
			Box::new(TransferType::LocalReserve),
			Box::new(VersionedAssetId::from(asset_id.clone())),
			Box::new(TransferType::LocalReserve),
			Box::new(VersionedXcm::from(custom_xcm)),
			Unlimited,
		));

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Transferred{ .. }) => {},]
		);
	});

	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		// Check that the transfer token back to Ethereum message was queue in the Ethereum
		// Outbound Queue
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumOutboundQueue(pezsnowbridge_pezpallet_outbound_queue::Event::MessageQueued{ .. }) => {},]
		);
	});

	// Send token back from Ethereum
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		let asset_id_after_reanchor: Location =
			Location::new(1, [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH))]);
		let token_id = TokenIdOf::convert_location(&asset_id_after_reanchor).unwrap();
		let message = VersionedMessage::V1(MessageV1 {
			chain_id: SEPOLIA_ID,
			command: Command::SendNativeToken {
				token_id,
				destination: Destination::AccountId32 { id: AssetHubZagrosReceiver::get().into() },
				amount: initial_fund / 10,
				fee: XCM_FEE,
			},
		});
		let (xcm, _) = EthereumInboundQueue::do_convert([0; 32].into(), message).unwrap();
		let _ = EthereumInboundQueue::send_xcm(xcm, AssetHubZagros::para_id().into()).unwrap();
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubZagros,
			vec![RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited{..}) => {},]
		);

		let events = AssetHubZagros::events();

		// Check that the native token burnt from reserved account
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Withdrawn { who, .. })
					if *who == ethereum_sovereign.clone(),
			)),
			"token burnt from Ethereum sovereign account."
		);

		// Check that the token was minted to beneficiary
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Deposited { who, .. })
					if *who == AssetHubZagrosReceiver::get()
			)),
			"Token minted to beneficiary."
		);
	});
}

#[test]
fn register_pna_in_v5_while_transfer_in_v4_should_work() {
	let assethub_sovereign = BridgeHubZagros::sovereign_account_id_of(
		BridgeHubZagros::sibling_location_of(AssetHubZagros::para_id()),
	);
	BridgeHubZagros::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);

	let asset_id: Location = Location { parents: 1, interior: [].into() };
	let expected_asset_id: Location =
		Location { parents: 1, interior: [GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH))].into() };

	let _expected_token_id = TokenIdOf::convert_location(&expected_asset_id).unwrap();

	let ethereum_sovereign: AccountId = pezsnowbridge_sovereign();

	// Register token in V5
	BridgeHubZagros::execute_with(|| {
		type RuntimeOrigin = <BridgeHubZagros as Chain>::RuntimeOrigin;
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		assert_ok!(<BridgeHubZagros as BridgeHubZagrosPallet>::Balances::force_set_balance(
			RuntimeOrigin::root(),
			pezsp_runtime::MultiAddress::Id(BridgeHubZagrosSender::get()),
			INITIAL_FUND * 10,
		));

		assert_ok!(<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::from(asset_id.clone())),
			AssetMetadata {
				name: "wnd".as_bytes().to_vec().try_into().unwrap(),
				symbol: "wnd".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
		// Check that a message was sent to Ethereum to create the agent
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumSystem(pezsnowbridge_pezpallet_system::Event::RegisterToken { .. }) => {},]
		);
	});

	AssetHubZagros::force_xcm_version(bridge_hub(), 4);
	AssetHubZagros::force_xcm_version(ethereum(), 4);
	AssetHubZagros::force_default_xcm_version(Some(4));
	BridgeHubZagros::force_default_xcm_version(Some(4));

	// Send token to Ethereum in V4 fomat
	AssetHubZagros::execute_with(|| {
		// LTS is V4
		use xcm::lts::{Junction::*, NetworkId::*, *};
		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		let assets = vec![Asset {
			id: AssetId(Location::parent()),
			fun: Fungibility::try_from(Fungible(TOKEN_AMOUNT)).unwrap(),
		}];
		let versioned_assets = VersionedAssets::V4(Assets::from(assets));

		let destination = VersionedLocation::V4(Location::new(
			2,
			[GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })],
		));

		let beneficiary = Location::new(
			0,
			[AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }],
		);

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
			RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			Box::new(destination),
			Box::new(versioned_assets),
			Box::new(TransferType::LocalReserve),
			Box::new(VersionedAssetId::V4(AssetId(Location::parent()))),
			Box::new(TransferType::LocalReserve),
			Box::new(VersionedXcm::V4(
				Xcm::<()>::builder_unsafe()
					.deposit_asset(WildAsset::AllCounted(1), beneficiary)
					.build()
			)),
			Unlimited,
		));

		let events = AssetHubZagros::events();
		// Check that the native asset transferred to some reserved account(sovereign of Ethereum)
		assert!(
			events.iter().any(|event| matches!(
				event,
				RuntimeEvent::Balances(pezpallet_balances::Event::Transfer { amount, to, ..})
					if *amount == TOKEN_AMOUNT && *to == ethereum_sovereign.clone(),
			)),
			"native token reserved to Ethereum sovereign account."
		);
	});

	// Check that the transfer token back to Ethereum message was queue in the Ethereum
	// Outbound Queue
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumOutboundQueue(pezsnowbridge_pezpallet_outbound_queue::Event::MessageQueued{ .. }) => {},]
		);
	});
}
