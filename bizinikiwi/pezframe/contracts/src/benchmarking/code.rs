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

//! Functions to procedurally construct contract code used for benchmarking.
//!
//! In order to be able to benchmark events that are triggered by contract execution
//! (API calls into seal, individual instructions), we need to generate contracts that
//! perform those events. Because those contracts can get very big we cannot simply define
//! them as text (.wat) as this will be too slow and consume too much memory. Therefore
//! we define this simple definition of a contract that can be passed to `create_code` that
//! compiles it down into a `WasmModule` that can be used as a contract's code.

use crate::Config;
use alloc::{vec, vec::Vec};
use pezframe_support::traits::Get;
use pezsp_runtime::{traits::Hash, Saturating};
use wasm_encoder::{BlockType, Instruction, ValType};

pub use super::module_definition::{
	body, DataSegment, ImportedFunction, ImportedMemory, ModuleDefinition,
};

/// The location where to put the generated code.
pub enum Location {
	/// Generate all code into the `call` exported function.
	Call,
	/// Generate all code into the `deploy` exported function.
	Deploy,
}

impl ImportedMemory {
	pub fn max<T: Config>() -> Self {
		let pages = max_pages::<T>();
		Self { min_pages: pages, max_pages: pages }
	}
}

/// A wasm module ready to be put on chain.
#[derive(Clone)]
pub struct WasmModule<T: Config> {
	pub code: Vec<u8>,
	pub hash: <T::Hashing as Hash>::Output,
}

impl<T: Config> From<ModuleDefinition> for WasmModule<T> {
	fn from(def: ModuleDefinition) -> Self {
		let code = super::module_definition::encode(def);
		let hash = T::Hashing::hash(&code);
		Self { code, hash }
	}
}

impl<T: Config> WasmModule<T> {
	/// Creates a wasm module with an empty `call` and `deploy` function and nothing else.
	pub fn dummy() -> Self {
		ModuleDefinition::default().into()
	}

	/// Same as `dummy` but with maximum sized linear memory and a dummy section of specified size.
	pub fn dummy_with_bytes(dummy_bytes: u32) -> Self {
		// We want the module to have the size `dummy_bytes`.
		// This is not completely correct as the overhead grows when the contract grows
		// because of variable length integer encoding. However, it is good enough to be that
		// close for benchmarking purposes.
		let module_overhead = 65;
		ModuleDefinition {
			memory: Some(ImportedMemory::max::<T>()),
			dummy_section: dummy_bytes.saturating_sub(module_overhead),
			..Default::default()
		}
		.into()
	}

	/// Creates a wasm module of `target_bytes` size. Used to benchmark the performance of
	/// `instantiate_with_code` for different sizes of wasm modules. The generated module maximizes
	/// instrumentation runtime by nesting blocks as deeply as possible given the byte budget.
	/// `code_location`: Whether to place the code into `deploy` or `call`.
	pub fn sized(target_bytes: u32, code_location: Location, use_float: bool) -> Self {
		use Instruction::{End, If, LocalGet, Return};
		// Base size of a contract is 63 bytes and each expansion adds 6 bytes.
		// We do one expansion less to account for the code section and function body
		// size fields inside the binary wasm module representation which are leb128 encoded
		// and therefore grow in size when the contract grows. We are not allowed to overshoot
		// because of the maximum code size that is enforced by `instantiate_with_code`.
		let mut expansions = (target_bytes.saturating_sub(63) / 6).saturating_sub(1);
		const EXPANSION: [Instruction<'static>; 4] =
			[LocalGet(0), If(BlockType::Empty), Return, End];
		let mut locals = vec![(1, ValType::I32)];
		if use_float {
			locals.push((1, ValType::F32));
			locals.push((2, ValType::F32));
			locals.push((3, ValType::F32));
			expansions.saturating_dec();
		}
		let mut module =
			ModuleDefinition { memory: Some(ImportedMemory::max::<T>()), ..Default::default() };
		let body = Some(body::repeated_with_locals(&locals, expansions, &EXPANSION));
		match code_location {
			Location::Call => module.call_body = body,
			Location::Deploy => module.deploy_body = body,
		}
		module.into()
	}

	/// Creates a wasm module that calls the imported function `noop` `repeat` times.
	pub fn noop(repeat: u32) -> Self {
		let pages = max_pages::<T>();
		ModuleDefinition {
			memory: Some(ImportedMemory::max::<T>()),
			imported_functions: vec![ImportedFunction {
				module: "seal0",
				name: "noop",
				params: vec![],
				return_type: None,
			}],
			// Write the output buffer size. The output size will be overwritten by the
			// supervisor with the real size when calling the getter. Since this size does not
			// change between calls it suffices to start with an initial value and then just
			// leave as whatever value was written there.
			data_segments: vec![DataSegment {
				offset: 0,
				value: (pages * 64 * 1024 - 4).to_le_bytes().to_vec(),
			}],
			call_body: Some(body::repeated(
				repeat,
				&[
					Instruction::Call(0), // call the imported function
				],
			)),
			..Default::default()
		}
		.into()
	}
}

/// The maximum amount of pages any contract is allowed to have according to the current `Schedule`.
pub fn max_pages<T: Config>() -> u32 {
	T::Schedule::get().limits.memory_pages
}
