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

//! # Tests for the Pezkuwichain runtime.

use super::*;
use crate::genesis_config_presets::PEZ_ASSET_ID;
use crate::{CENTS, MILLICENTS};
use pezsp_runtime::traits::Zero;
use pezsp_weights::WeightToFee;
use testnet_teyrchains_constants::pezkuwichain::fee;

/// We can fit at least 1000 transfers in a block.
#[test]
fn sane_block_weight() {
	use pezpallet_balances::WeightInfo;
	let block = RuntimeBlockWeights::get().max_block;
	let base = RuntimeBlockWeights::get().get(DispatchClass::Normal).base_extrinsic;
	let transfer =
		base + weights::pezpallet_balances::WeightInfo::<Runtime>::transfer_allow_death();

	let fit = block.checked_div_per_component(&transfer).unwrap_or_default();
	assert!(fit >= 1000, "{} should be at least 1000", fit);
}

/// The fee for one transfer is at most 1 CENT.
#[test]
fn sane_transfer_fee() {
	use pezpallet_balances::WeightInfo;
	let base = RuntimeBlockWeights::get().get(DispatchClass::Normal).base_extrinsic;
	let transfer =
		base + weights::pezpallet_balances::WeightInfo::<Runtime>::transfer_allow_death();

	let fee: Balance = fee::WeightToFee::weight_to_fee(&transfer);
	assert!(fee <= CENTS, "{} MILLICENTS should be at most 1000", fee / MILLICENTS);
}

/// Weight is being charged for both dimensions.
#[test]
fn weight_charged_for_both_components() {
	let fee: Balance = fee::WeightToFee::weight_to_fee(&Weight::from_parts(10_000, 0));
	assert!(!fee.is_zero(), "Charges for ref time");

	let fee: Balance = fee::WeightToFee::weight_to_fee(&Weight::from_parts(0, 10_000));
	assert_eq!(fee, CENTS, "10kb maps to CENT");
}

/// Filling up a block by proof size is at most 30 times more expensive than ref time.
///
/// This is just a sanity check.
#[test]
fn full_block_fee_ratio() {
	let block = RuntimeBlockWeights::get().max_block;
	let time_fee: Balance =
		fee::WeightToFee::weight_to_fee(&Weight::from_parts(block.ref_time(), 0));
	let proof_fee: Balance =
		fee::WeightToFee::weight_to_fee(&Weight::from_parts(0, block.proof_size()));

	let proof_o_time = proof_fee.checked_div(time_fee).unwrap_or_default();
	assert!(proof_o_time <= 30, "{} should be at most 30", proof_o_time);
	let time_o_proof = time_fee.checked_div(proof_fee).unwrap_or_default();
	assert!(time_o_proof <= 30, "{} should be at most 30", time_o_proof);
}

// =============================================================================
// THE PEZ ASSET TEAM
// =============================================================================

/// The genesis preset must hand PEZ to an account nobody can sign as.
///
/// `pallet-assets` gives the owner named at genesis all four roles -- owner, issuer, admin
/// and freezer -- and two of those are not bookkeeping: the issuer creates tokens and the
/// admin moves them out of accounts it does not hold. On a token whose entire point is a
/// fixed five billion, and on a chain whose treasury sits in a keyless pot, neither may
/// belong to a key.
///
/// This reads the preset the chain is actually built from, not a hand-written copy of it.
#[test]
fn the_pez_asset_team_is_keyless_in_every_preset() {
	use pezsp_runtime::traits::AccountIdConversion;

	let expected: AccountId = PezAssetTeamId::get().into_account_truncating();

	// Building a preset reaches for host functions, so it needs an externalities environment.
	pezsp_io::TestExternalities::default().execute_with(|| {
		for preset in [
			Some(pezsp_genesis_builder::PresetId::from("genesis")),
			Some(pezsp_genesis_builder::PresetId::from(
				pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET,
			)),
			Some(pezsp_genesis_builder::PresetId::from(pezsp_genesis_builder::DEV_RUNTIME_PRESET)),
		] {
			let id = preset.expect("named preset");
			let raw = crate::genesis_config_presets::get_preset(&id)
				.unwrap_or_else(|| panic!("preset {id:?} exists"));
			let json: serde_json::Value =
				serde_json::from_slice(&raw).expect("the preset is valid json");

			let assets = json["assets"]["assets"]
				.as_array()
				.unwrap_or_else(|| panic!("preset {id:?} declares assets"));

			let pez = assets
				.iter()
				.find(|entry| entry[0] == serde_json::json!(PEZ_ASSET_ID))
				.unwrap_or_else(|| panic!("preset {id:?} creates PEZ"));

			let owner: AccountId =
				serde_json::from_value(pez[1].clone()).expect("the owner decodes");

			assert_eq!(
				owner, expected,
				"preset {id:?} gives PEZ an owner that can be signed for; \
			 the issuer could then mint past the fixed supply and the admin could \
			 force-transfer out of the treasury pot"
			);
		}
	});
}
