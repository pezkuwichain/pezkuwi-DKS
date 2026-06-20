// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>

use codec::Encode;
use pezframe_support::{
	assert_err, assert_ok,
	traits::{fungible::Mutate, OnFinalize, OnInitialize},
};
use pezframe_system::pezpallet_prelude::BlockNumberFor;
use pezsnowbridge_core::{ChannelId, ParaId};
use pezsnowbridge_pezpallet_ethereum_client_fixtures::*;
use pezsp_core::{Get, H160, U256};
use pezsp_keyring::Sr25519Keyring::*;
use pezsp_runtime::{traits::Header, AccountId32, DigestItem, SaturatedConversion, Saturating};
use teyrchains_runtimes_test_utils::{
	AccountIdOf, BalanceOf, CollatorSessionKeys, ExtBuilder, ValidatorIdOf, XcmReceivedFrom,
};
use xcm::latest::prelude::*;
use xcm_executor::XcmExecutor;

type RuntimeHelper<Runtime, AllPalletsWithoutSystem = ()> =
	teyrchains_runtimes_test_utils::RuntimeHelper<Runtime, AllPalletsWithoutSystem>;

pub fn initial_fund<Runtime>(assethub_teyrchain_id: u32, initial_amount: u128)
where
	Runtime: pezframe_system::Config + pezpallet_balances::Config,
{
	// fund asset hub sovereign account enough so it can pay fees
	let asset_hub_sovereign_account =
		pezsnowbridge_core::sibling_sovereign_account::<Runtime>(assethub_teyrchain_id.into());
	<pezpallet_balances::Pezpallet<Runtime>>::mint_into(
		&asset_hub_sovereign_account,
		initial_amount.saturated_into::<BalanceOf<Runtime>>(),
	)
	.unwrap();
}

pub fn send_transfer_token_message<Runtime, XcmConfig>(
	ethereum_chain_id: u64,
	assethub_teyrchain_id: u32,
	weth_contract_address: H160,
	destination_address: H160,
	fee_amount: u128,
) -> Outcome
where
	Runtime: pezframe_system::Config
		+ pezpallet_balances::Config
		+ pezpallet_session::Config
		+ pezpallet_xcm::Config
		+ teyrchain_info::Config
		+ pezpallet_collator_selection::Config
		+ pezcumulus_pezpallet_teyrchain_system::Config
		+ pezsnowbridge_pezpallet_outbound_queue::Config
		+ pezpallet_timestamp::Config,
	XcmConfig: xcm_executor::Config,
{
	let assethub_teyrchain_location = Location::new(1, Teyrchain(assethub_teyrchain_id));
	let asset = Asset {
		id: AssetId(Location::new(
			0,
			[AccountKey20 { network: None, key: weth_contract_address.into() }],
		)),
		fun: Fungible(1000000000),
	};
	let assets = vec![asset.clone()];

	let inner_xcm = Xcm(vec![
		WithdrawAsset(Assets::from(assets.clone())),
		ClearOrigin,
		BuyExecution { fees: asset, weight_limit: Unlimited },
		DepositAsset {
			assets: Wild(All),
			beneficiary: Location::new(
				0,
				[AccountKey20 { network: None, key: destination_address.into() }],
			),
		},
		SetTopic([0; 32]),
	]);

	let fee =
		Asset { id: AssetId(Location { parents: 1, interior: Here }), fun: Fungible(fee_amount) };

	// prepare transfer token message
	let xcm = Xcm(vec![
		WithdrawAsset(Assets::from(vec![fee.clone()])),
		BuyExecution { fees: fee, weight_limit: Unlimited },
		ExportMessage {
			network: Ethereum { chain_id: ethereum_chain_id },
			destination: Here,
			xcm: inner_xcm,
		},
	]);

	// execute XCM
	let mut hash = xcm.using_encoded(pezsp_io::hashing::blake2_256);
	XcmExecutor::<XcmConfig>::prepare_and_execute(
		assethub_teyrchain_location,
		xcm,
		&mut hash,
		RuntimeHelper::<Runtime>::xcm_max_weight(XcmReceivedFrom::Sibling),
		Weight::zero(),
	)
}

pub fn send_transfer_token_message_success<Runtime, XcmConfig>(
	ethereum_chain_id: u64,
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	assethub_teyrchain_id: u32,
	weth_contract_address: H160,
	destination_address: H160,
	fee_amount: u128,
	pezsnowbridge_pezpallet_outbound_queue: Box<
		dyn Fn(Vec<u8>) -> Option<pezsnowbridge_pezpallet_outbound_queue::Event<Runtime>>,
	>,
) where
	Runtime: pezframe_system::Config
		+ pezpallet_balances::Config
		+ pezpallet_session::Config
		+ pezpallet_xcm::Config
		+ teyrchain_info::Config
		+ pezpallet_collator_selection::Config
		+ pezpallet_message_queue::Config
		+ pezcumulus_pezpallet_teyrchain_system::Config
		+ pezsnowbridge_pezpallet_outbound_queue::Config
		+ pezsnowbridge_pezpallet_system::Config
		+ pezpallet_timestamp::Config,
	XcmConfig: xcm_executor::Config,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	<Runtime as pezframe_system::Config>::AccountId: From<pezsp_runtime::AccountId32> + AsRef<[u8]>,
{
	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_key.collators())
		.with_session_keys(collator_session_key.session_keys())
		.with_para_id(runtime_para_id.into())
		.with_tracing()
		.build()
		.execute_with(|| {
			<pezsnowbridge_pezpallet_system::Pezpallet<Runtime>>::initialize(
				runtime_para_id.into(),
				assethub_teyrchain_id.into(),
			)
			.unwrap();

			// fund asset hub sovereign account enough so it can pay fees
			initial_fund::<Runtime>(assethub_teyrchain_id, 5_000_000_000_000);

			let outcome = send_transfer_token_message::<Runtime, XcmConfig>(
				ethereum_chain_id,
				assethub_teyrchain_id,
				weth_contract_address,
				destination_address,
				fee_amount,
			);

			assert_ok!(outcome.ensure_complete());

			// check events
			let mut events = <pezframe_system::Pezpallet<Runtime>>::events()
				.into_iter()
				.filter_map(|e| pezsnowbridge_pezpallet_outbound_queue(e.event.encode()));
			assert!(events.any(|e| matches!(
				e,
				pezsnowbridge_pezpallet_outbound_queue::Event::MessageQueued { .. }
			)));

			let block_number = <pezframe_system::Pezpallet<Runtime>>::block_number();
			let next_block_number = <pezframe_system::Pezpallet<Runtime>>::block_number()
				.saturating_add(BlockNumberFor::<Runtime>::from(1u32));

			// finish current block
			<pezpallet_message_queue::Pezpallet<Runtime>>::on_finalize(block_number);
			<pezsnowbridge_pezpallet_outbound_queue::Pezpallet<Runtime>>::on_finalize(block_number);
			<pezframe_system::Pezpallet<Runtime>>::on_finalize(block_number);

			// start next block
			<pezframe_system::Pezpallet<Runtime>>::set_block_number(next_block_number);
			<pezframe_system::Pezpallet<Runtime>>::on_initialize(next_block_number);
			<pezsnowbridge_pezpallet_outbound_queue::Pezpallet<Runtime>>::on_initialize(
				next_block_number,
			);
			<pezpallet_message_queue::Pezpallet<Runtime>>::on_initialize(next_block_number);

			// finish next block
			<pezpallet_message_queue::Pezpallet<Runtime>>::on_finalize(next_block_number);
			<pezsnowbridge_pezpallet_outbound_queue::Pezpallet<Runtime>>::on_finalize(
				next_block_number,
			);
			let included_head = <pezframe_system::Pezpallet<Runtime>>::finalize();

			let origin: ParaId = assethub_teyrchain_id.into();
			let channel_id: ChannelId = origin.into();

			let nonce =
				pezsnowbridge_pezpallet_outbound_queue::Nonce::<Runtime>::try_get(channel_id);
			assert_ok!(nonce);
			assert_eq!(nonce.unwrap(), 1);

			let digest = included_head.digest();

			let digest_items = digest.logs();
			assert!(digest_items.len() == 1 && digest_items[0].as_other().is_some());
		});
}

pub fn ethereum_outbound_queue_processes_messages_before_message_queue_works<
	Runtime,
	XcmConfig,
	AllPalletsWithoutSystem,
>(
	ethereum_chain_id: u64,
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	assethub_teyrchain_id: u32,
	weth_contract_address: H160,
	destination_address: H160,
	fee_amount: u128,
	pezsnowbridge_pezpallet_outbound_queue: Box<
		dyn Fn(Vec<u8>) -> Option<pezsnowbridge_pezpallet_outbound_queue::Event<Runtime>>,
	>,
) where
	Runtime: pezframe_system::Config
		+ pezpallet_balances::Config
		+ pezpallet_session::Config
		+ pezpallet_xcm::Config
		+ teyrchain_info::Config
		+ pezpallet_collator_selection::Config
		+ pezpallet_message_queue::Config
		+ pezcumulus_pezpallet_teyrchain_system::Config
		+ pezsnowbridge_pezpallet_outbound_queue::Config
		+ pezsnowbridge_pezpallet_system::Config
		+ pezpallet_timestamp::Config,
	XcmConfig: xcm_executor::Config,
	AllPalletsWithoutSystem:
		OnInitialize<BlockNumberFor<Runtime>> + OnFinalize<BlockNumberFor<Runtime>>,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	<Runtime as pezframe_system::Config>::AccountId: From<pezsp_runtime::AccountId32> + AsRef<[u8]>,
{
	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_key.collators())
		.with_session_keys(collator_session_key.session_keys())
		.with_para_id(runtime_para_id.into())
		.with_tracing()
		.build()
		.execute_with(|| {
			<pezsnowbridge_pezpallet_system::Pezpallet<Runtime>>::initialize(
				runtime_para_id.into(),
				assethub_teyrchain_id.into(),
			)
			.unwrap();

			// fund asset hub sovereign account enough so it can pay fees
			initial_fund::<Runtime>(assethub_teyrchain_id, 5_000_000_000_000);

			let outcome = send_transfer_token_message::<Runtime, XcmConfig>(
				ethereum_chain_id,
				assethub_teyrchain_id,
				weth_contract_address,
				destination_address,
				fee_amount,
			);

			assert_ok!(outcome.ensure_complete());

			// check events
			let mut events = <pezframe_system::Pezpallet<Runtime>>::events()
				.into_iter()
				.filter_map(|e| pezsnowbridge_pezpallet_outbound_queue(e.event.encode()));
			assert!(events.any(|e| matches!(
				e,
				pezsnowbridge_pezpallet_outbound_queue::Event::MessageQueued { .. }
			)));

			let next_block_number: U256 = <pezframe_system::Pezpallet<Runtime>>::block_number()
				.saturating_add(BlockNumberFor::<Runtime>::from(1u32))
				.into();

			let included_head =
				RuntimeHelper::<Runtime, AllPalletsWithoutSystem>::run_to_block_with_finalize(
					next_block_number.as_u32(),
				);
			let digest = included_head.digest();
			let digest_items = digest.logs();

			let mut found_outbound_digest = false;
			for digest_item in digest_items {
				match digest_item {
					DigestItem::Other(_) => found_outbound_digest = true,
					_ => {},
				}
			}

			assert_eq!(found_outbound_digest, true);
		});
}

pub fn send_unpaid_transfer_token_message<Runtime, XcmConfig>(
	ethereum_chain_id: u64,
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	assethub_teyrchain_id: u32,
	weth_contract_address: H160,
	destination_contract: H160,
) where
	Runtime: pezframe_system::Config
		+ pezpallet_balances::Config
		+ pezpallet_session::Config
		+ pezpallet_xcm::Config
		+ teyrchain_info::Config
		+ pezpallet_collator_selection::Config
		+ pezcumulus_pezpallet_teyrchain_system::Config
		+ pezsnowbridge_pezpallet_outbound_queue::Config
		+ pezsnowbridge_pezpallet_system::Config
		+ pezpallet_timestamp::Config,
	XcmConfig: xcm_executor::Config,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
{
	let assethub_teyrchain_location = Location::new(1, Teyrchain(assethub_teyrchain_id));

	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_key.collators())
		.with_session_keys(collator_session_key.session_keys())
		.with_para_id(runtime_para_id.into())
		.with_tracing()
		.build()
		.execute_with(|| {
			<pezsnowbridge_pezpallet_system::Pezpallet<Runtime>>::initialize(
				runtime_para_id.into(),
				assethub_teyrchain_id.into(),
			)
			.unwrap();

			let asset = Asset {
				id: AssetId(Location::new(
					0,
					[AccountKey20 { network: None, key: weth_contract_address.into() }],
				)),
				fun: Fungible(1000000000),
			};
			let assets = vec![asset.clone()];

			let inner_xcm = Xcm(vec![
				WithdrawAsset(Assets::from(assets.clone())),
				ClearOrigin,
				BuyExecution { fees: asset, weight_limit: Unlimited },
				DepositAsset {
					assets: Wild(AllCounted(1)),
					beneficiary: Location::new(
						0,
						[AccountKey20 { network: None, key: destination_contract.into() }],
					),
				},
				SetTopic([0; 32]),
			]);

			// prepare transfer token message
			let xcm = Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				ExportMessage {
					network: Ethereum { chain_id: ethereum_chain_id },
					destination: Here,
					xcm: inner_xcm,
				},
			]);

			// execute XCM
			let mut hash = xcm.using_encoded(pezsp_io::hashing::blake2_256);
			let outcome = XcmExecutor::<XcmConfig>::prepare_and_execute(
				assethub_teyrchain_location,
				xcm,
				&mut hash,
				RuntimeHelper::<Runtime>::xcm_max_weight(XcmReceivedFrom::Sibling),
				Weight::zero(),
			);
			assert_ok!(outcome.ensure_complete());
		});
}

#[allow(clippy::too_many_arguments)]
pub fn send_transfer_token_message_failure<Runtime, XcmConfig>(
	ethereum_chain_id: u64,
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	assethub_teyrchain_id: u32,
	initial_amount: u128,
	weth_contract_address: H160,
	destination_address: H160,
	fee_amount: u128,
	expected_error: XcmError,
) where
	Runtime: pezframe_system::Config
		+ pezpallet_balances::Config
		+ pezpallet_session::Config
		+ pezpallet_xcm::Config
		+ teyrchain_info::Config
		+ pezpallet_collator_selection::Config
		+ pezcumulus_pezpallet_teyrchain_system::Config
		+ pezsnowbridge_pezpallet_outbound_queue::Config
		+ pezsnowbridge_pezpallet_system::Config
		+ pezpallet_timestamp::Config,
	XcmConfig: xcm_executor::Config,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
{
	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_key.collators())
		.with_session_keys(collator_session_key.session_keys())
		.with_para_id(runtime_para_id.into())
		.with_tracing()
		.build()
		.execute_with(|| {
			<pezsnowbridge_pezpallet_system::Pezpallet<Runtime>>::initialize(
				runtime_para_id.into(),
				assethub_teyrchain_id.into(),
			)
			.unwrap();

			// fund asset hub sovereign account enough so it can pay fees
			initial_fund::<Runtime>(assethub_teyrchain_id, initial_amount);

			let outcome = send_transfer_token_message::<Runtime, XcmConfig>(
				ethereum_chain_id,
				assethub_teyrchain_id,
				weth_contract_address,
				destination_address,
				fee_amount,
			);
			assert_err!(
				outcome.ensure_complete(),
				InstructionError { index: 0, error: expected_error }
			);
		});
}

pub fn ethereum_extrinsic<Runtime>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	construct_and_apply_extrinsic: fn(
		pezsp_keyring::Sr25519Keyring,
		<Runtime as pezframe_system::Config>::RuntimeCall,
	) -> pezsp_runtime::DispatchOutcome,
) where
	Runtime: pezframe_system::Config
		+ pezpallet_balances::Config
		+ pezpallet_session::Config
		+ pezpallet_xcm::Config
		+ pezpallet_utility::Config
		+ teyrchain_info::Config
		+ pezpallet_collator_selection::Config
		+ pezcumulus_pezpallet_teyrchain_system::Config
		+ pezsnowbridge_pezpallet_outbound_queue::Config
		+ pezsnowbridge_pezpallet_system::Config
		+ pezsnowbridge_pezpallet_ethereum_client::Config
		+ pezpallet_timestamp::Config,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	<Runtime as pezpallet_utility::Config>::RuntimeCall:
		From<pezsnowbridge_pezpallet_ethereum_client::Call<Runtime>>,
	<Runtime as pezframe_system::Config>::RuntimeCall: From<pezpallet_utility::Call<Runtime>>,
	AccountIdOf<Runtime>: From<AccountId32>,
{
	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_key.collators())
		.with_session_keys(collator_session_key.session_keys())
		.with_para_id(runtime_para_id.into())
		.with_tracing()
		.build()
		.execute_with(|| {
			let initial_checkpoint = make_checkpoint();
			let update = make_finalized_header_update();
			let sync_committee_update = make_sync_committee_update();
			let mut invalid_update = make_finalized_header_update();
			let mut invalid_sync_committee_update = make_sync_committee_update();
			invalid_update.finalized_header.slot = 4354;
			invalid_sync_committee_update.finalized_header.slot = 4354;

			let alice = Alice;
			let alice_account = alice.to_account_id();
			<pezpallet_balances::Pezpallet<Runtime>>::mint_into(
				&alice_account.clone().into(),
				10_000_000_000_000_u128.saturated_into::<BalanceOf<Runtime>>(),
			)
			.unwrap();
			let balance_before = <pezpallet_balances::Pezpallet<Runtime>>::free_balance(
				&alice_account.clone().into(),
			);

			assert_ok!(
				<pezsnowbridge_pezpallet_ethereum_client::Pezpallet<Runtime>>::force_checkpoint(
					RuntimeHelper::<Runtime>::root_origin(),
					initial_checkpoint.clone(),
				)
			);
			let balance_after_checkpoint = <pezpallet_balances::Pezpallet<Runtime>>::free_balance(
				&alice_account.clone().into(),
			);

			let update_call: <Runtime as pezpallet_utility::Config>::RuntimeCall =
				pezsnowbridge_pezpallet_ethereum_client::Call::<Runtime>::submit {
					update: Box::new(*update.clone()),
				}
				.into();

			let invalid_update_call: <Runtime as pezpallet_utility::Config>::RuntimeCall =
				pezsnowbridge_pezpallet_ethereum_client::Call::<Runtime>::submit {
					update: Box::new(*invalid_update),
				}
				.into();

			let update_sync_committee_call: <Runtime as pezpallet_utility::Config>::RuntimeCall =
				pezsnowbridge_pezpallet_ethereum_client::Call::<Runtime>::submit {
					update: Box::new(*sync_committee_update),
				}
				.into();

			let invalid_update_sync_committee_call: <Runtime as pezpallet_utility::Config>::RuntimeCall =
				pezsnowbridge_pezpallet_ethereum_client::Call::<Runtime>::submit {
					update: Box::new(*invalid_sync_committee_update),
				}
					.into();

			// Finalized header update
			let update_outcome = construct_and_apply_extrinsic(alice, update_call.into());
			assert_ok!(update_outcome);
			let balance_after_update = <pezpallet_balances::Pezpallet<Runtime>>::free_balance(
				&alice_account.clone().into(),
			);

			// All the extrinsics in this test do no fit into 1 block
			let _ = RuntimeHelper::<Runtime>::run_to_block(2, alice_account.clone().into());

			// Invalid finalized header update
			let invalid_update_outcome =
				construct_and_apply_extrinsic(alice, invalid_update_call.into());
			assert_err!(
				invalid_update_outcome,
				pezsnowbridge_pezpallet_ethereum_client::Error::<Runtime>::InvalidUpdateSlot
			);
			let balance_after_invalid_update =
				<pezpallet_balances::Pezpallet<Runtime>>::free_balance(
					&alice_account.clone().into(),
				);

			// Sync committee update
			let sync_committee_outcome =
				construct_and_apply_extrinsic(alice, update_sync_committee_call.into());
			assert_ok!(sync_committee_outcome);
			let balance_after_sync_com_update =
				<pezpallet_balances::Pezpallet<Runtime>>::free_balance(
					&alice_account.clone().into(),
				);

			// Invalid sync committee update
			let invalid_sync_committee_outcome =
				construct_and_apply_extrinsic(alice, invalid_update_sync_committee_call.into());
			assert_err!(
				invalid_sync_committee_outcome,
				pezsnowbridge_pezpallet_ethereum_client::Error::<Runtime>::InvalidUpdateSlot
			);
			let balance_after_invalid_sync_com_update =
				<pezpallet_balances::Pezpallet<Runtime>>::free_balance(
					&alice_account.clone().into(),
				);

			// Assert paid operations are charged and free operations are free
			// Checkpoint is a free operation
			assert!(balance_before == balance_after_checkpoint);
			let gap =
				<Runtime as pezsnowbridge_pezpallet_ethereum_client::Config>::FreeHeadersInterval::get();
			// Large enough header gap is free
			if update.finalized_header.slot >= initial_checkpoint.header.slot + gap as u64 {
				assert!(balance_after_checkpoint == balance_after_update);
			} else {
				// Otherwise paid
				assert!(balance_after_checkpoint > balance_after_update);
			}
			// An invalid update is paid
			assert!(balance_after_update > balance_after_invalid_update);
			// A successful sync committee update is free
			assert!(balance_after_invalid_update == balance_after_sync_com_update);
			// An invalid sync committee update is paid
			assert!(balance_after_sync_com_update > balance_after_invalid_sync_com_update);
		});
}

pub fn ethereum_to_pezkuwi_message_extrinsics_work<Runtime>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	construct_and_apply_extrinsic: fn(
		pezsp_keyring::Sr25519Keyring,
		<Runtime as pezframe_system::Config>::RuntimeCall,
	) -> pezsp_runtime::DispatchOutcome,
) where
	Runtime: pezframe_system::Config
		+ pezpallet_balances::Config
		+ pezpallet_session::Config
		+ pezpallet_xcm::Config
		+ pezpallet_utility::Config
		+ teyrchain_info::Config
		+ pezpallet_collator_selection::Config
		+ pezcumulus_pezpallet_teyrchain_system::Config
		+ pezsnowbridge_pezpallet_outbound_queue::Config
		+ pezsnowbridge_pezpallet_system::Config
		+ pezsnowbridge_pezpallet_ethereum_client::Config
		+ pezpallet_timestamp::Config,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	<Runtime as pezpallet_utility::Config>::RuntimeCall:
		From<pezsnowbridge_pezpallet_ethereum_client::Call<Runtime>>,
	<Runtime as pezframe_system::Config>::RuntimeCall: From<pezpallet_utility::Call<Runtime>>,
	AccountIdOf<Runtime>: From<AccountId32>,
{
	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_key.collators())
		.with_session_keys(collator_session_key.session_keys())
		.with_para_id(runtime_para_id.into())
		.with_tracing()
		.build()
		.execute_with(|| {
			let initial_checkpoint = make_checkpoint();
			let sync_committee_update = make_sync_committee_update();

			let alice = Alice;
			let alice_account = alice.to_account_id();
			<pezpallet_balances::Pezpallet<Runtime>>::mint_into(
				&alice_account.into(),
				10_000_000_000_000_u128.saturated_into::<BalanceOf<Runtime>>(),
			)
			.unwrap();

			assert_ok!(
				<pezsnowbridge_pezpallet_ethereum_client::Pezpallet<Runtime>>::force_checkpoint(
					RuntimeHelper::<Runtime>::root_origin(),
					initial_checkpoint,
				)
			);

			let update_sync_committee_call: <Runtime as pezpallet_utility::Config>::RuntimeCall =
				pezsnowbridge_pezpallet_ethereum_client::Call::<Runtime>::submit {
					update: Box::new(*sync_committee_update),
				}
				.into();

			let sync_committee_outcome =
				construct_and_apply_extrinsic(alice, update_sync_committee_call.into());
			assert_ok!(sync_committee_outcome);
		});
}
