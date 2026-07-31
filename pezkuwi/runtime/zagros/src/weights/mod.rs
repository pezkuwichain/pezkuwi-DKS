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

//! A list of the different weight modules for our runtime.

pub mod pezframe_system;
pub mod pezframe_system_extensions;
pub mod pezkuwi_runtime_common_assigned_slots;
pub mod pezkuwi_runtime_common_auctions;
pub mod pezkuwi_runtime_common_claims;
pub mod pezkuwi_runtime_common_crowdloan;
pub mod pezkuwi_runtime_common_paras_registrar;
pub mod pezkuwi_runtime_common_slots;
pub mod pezkuwi_runtime_teyrchains_configuration;
pub mod pezkuwi_runtime_teyrchains_coretime;
pub mod pezkuwi_runtime_teyrchains_disputes;
pub mod pezkuwi_runtime_teyrchains_hrmp;
pub mod pezkuwi_runtime_teyrchains_inclusion;
pub mod pezkuwi_runtime_teyrchains_initializer;
pub mod pezkuwi_runtime_teyrchains_on_demand;
pub mod pezkuwi_runtime_teyrchains_paras;
pub mod pezkuwi_runtime_teyrchains_paras_inherent;
pub mod pezpallet_balances_balances;
pub mod pezpallet_beefy_mmr;
pub mod pezpallet_conviction_voting;
pub mod pezpallet_indices;
pub mod pezpallet_message_queue;
pub mod pezpallet_migrations;
pub mod pezpallet_mmr;
pub mod pezpallet_multisig;
pub mod pezpallet_parameters;
pub mod pezpallet_preimage;
pub mod pezpallet_proxy;
pub mod pezpallet_referenda_referenda;
pub mod pezpallet_scheduler;
pub mod pezpallet_session;
pub mod pezpallet_sudo;
pub mod pezpallet_timestamp;
pub mod pezpallet_transaction_payment;
pub mod pezpallet_treasury;
pub mod pezpallet_utility;
pub mod pezpallet_vesting;
pub mod pezpallet_whitelist;
pub mod pezpallet_xcm;
pub mod xcm;
