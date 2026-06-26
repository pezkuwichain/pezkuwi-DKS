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

//! # FRAME Pezpallet Examples
//!
//! This crate contains a collection of simple examples of FRAME pallets, demonstrating useful
//! features in action. It is not intended to be used in production.
//!
//! ## Pallets
//!
//! - [`pezpallet_example_basic`]: This pezpallet demonstrates concepts, APIs and structures common
//!   to most FRAME runtimes.
//!
//! - [`pezpallet_example_offchain_worker`]: This pezpallet demonstrates concepts, APIs and
//!   structures common to most offchain workers.
//!
//! - [`pezpallet_default_config_example`]: This pezpallet demonstrates different ways to implement
//!   the `Config` trait of pallets.
//!
//! - [`pezpallet_dev_mode`]: This pezpallet demonstrates the ease of requirements for a pezpallet
//!   in "dev mode".
//!
//! - [`pezpallet_example_kitchensink`]: This pezpallet demonstrates a catalog of all FRAME macros
//!   in use and their various syntax options.
//!
//! - [`pezpallet_example_split`]: A simple example of a FRAME pezpallet demonstrating the ability
//!   to split sections across multiple files.
//!
//! - [`pezpallet_example_pezframe_crate`]: Example pezpallet showcasing how one can be built using
//!   only the
//! `pezframe` umbrella crate.
//!
//! - [`pezpallet_example_single_block_migrations`]: An example pezpallet demonstrating
//!   best-practices for writing storage migrations.
//!
//! - [`pezpallet_example_tasks`]: This pezpallet demonstrates the use of `Tasks` to execute service
//!   work.
//!
//! - [`pezpallet_example_view_functions`]: This pezpallet demonstrates the use of view functions to
//!   query pezpallet state.
//!
//! - [`pezpallet_example_authorization_tx_extension`]: An example `TransactionExtension` that
//!   authorizes a custom origin through signature validation, along with two support pallets to
//!   showcase the usage.
//!
//! **Tip**: Use `cargo doc --package <pezpallet-name> --open` to view each pezpallet's
//! documentation.
