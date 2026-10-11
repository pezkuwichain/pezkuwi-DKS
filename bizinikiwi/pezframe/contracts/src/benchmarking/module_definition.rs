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

//! A declarative description of the contract modules used for benchmarking, and its encoding.
//!
//! The encoding lays a module out exactly as the `parity-wasm` builder it replaced did, so the
//! benchmarked code -- and with it the sizes the benchmarks are parameterised by -- does not
//! change: the type section in first-use order with duplicates merged, the memory import before
//! the function imports, `deploy` and `call` as the first two defined functions, and the
//! `dummy` custom section last.

use alloc::{borrow::ToOwned, vec, vec::Vec};
use wasm_encoder::{
	CodeSection, ConstExpr, CustomSection, DataSection, ElementSection, Elements, EntityType,
	ExportKind, ExportSection, Function, FunctionSection, GlobalSection, GlobalType, ImportSection,
	Instruction, MemoryType, Module, RefType, TableSection, TableType, TypeSection, ValType,
};

/// A group of locals of one type: how many, and the type.
pub type Local = (u32, ValType);

/// A function body: its locals and its instructions, including the final `end`.
#[derive(Clone)]
pub struct FuncBody {
	pub locals: Vec<Local>,
	pub instructions: Vec<Instruction<'static>>,
}

impl FuncBody {
	/// A body without locals that does nothing.
	pub fn empty() -> Self {
		Self { locals: Vec::new(), instructions: vec![Instruction::End] }
	}
}

/// Pass to `create_code` in order to create a compiled `WasmModule`.
///
/// This exists to have a more declarative way to describe a wasm module than to use
/// an encoder directly. It is tailored to fit the structure of contracts that are
/// needed for benchmarking.
#[derive(Default)]
pub struct ModuleDefinition {
	/// Imported memory attached to the module. No memory is imported if `None`.
	pub memory: Option<ImportedMemory>,
	/// Initializers for the imported memory.
	pub data_segments: Vec<DataSegment>,
	/// Creates the supplied amount of i64 mutable globals initialized with random values.
	pub num_globals: u32,
	/// List of functions that the module should import. They start with index 0.
	pub imported_functions: Vec<ImportedFunction>,
	/// Function body of the exported `deploy` function. Body is empty if `None`.
	/// Its index is `imported_functions.len()`.
	pub deploy_body: Option<FuncBody>,
	/// Function body of the exported `call` function. Body is empty if `None`.
	/// Its index is `imported_functions.len() + 1`.
	pub call_body: Option<FuncBody>,
	/// Function body of a non-exported function with index `imported_functions.len() + 2`.
	pub aux_body: Option<FuncBody>,
	/// The amount of I64 arguments the aux function should have.
	pub aux_arg_num: u32,
	/// Create a table containing function pointers.
	pub table: Option<TableSegment>,
	/// Create a section named "dummy" of the specified size. This is useful in order to
	/// benchmark the overhead of loading and storing codes of specified sizes. The dummy
	/// section only contributes to the size of the contract but does not affect execution.
	pub dummy_section: u32,
}

pub struct TableSegment {
	/// How many elements should be created inside the table.
	pub num_elements: u32,
	/// The function index with which all table elements should be initialized.
	pub function_index: u32,
}

pub struct DataSegment {
	pub offset: u32,
	pub value: Vec<u8>,
}

#[derive(Clone)]
pub struct ImportedMemory {
	pub min_pages: u32,
	pub max_pages: u32,
}

pub struct ImportedFunction {
	pub module: &'static str,
	pub name: &'static str,
	pub params: Vec<ValType>,
	pub return_type: Option<ValType>,
}

/// Encodes `def` as a wasm module.
pub fn encode(def: ModuleDefinition) -> Vec<u8> {
	// internal functions start at that offset.
	let func_offset = u32::try_from(def.imported_functions.len()).unwrap();

	// Function types in order of first use; a repeated signature reuses the first.
	let mut types: Vec<(Vec<ValType>, Vec<ValType>)> = Vec::new();
	let mut type_index = |params: Vec<ValType>, results: Vec<ValType>| -> u32 {
		let signature = (params, results);
		let index = types.iter().position(|t| *t == signature).unwrap_or_else(|| {
			types.push(signature);
			types.len() - 1
		});
		u32::try_from(index).unwrap()
	};

	// Every contract must export "deploy" and "call" functions.
	let mut functions = vec![
		(type_index(vec![], vec![]), def.deploy_body.unwrap_or_else(FuncBody::empty)),
		(type_index(vec![], vec![]), def.call_body.unwrap_or_else(FuncBody::empty)),
	];

	// If specified we add an additional internal function
	if let Some(body) = def.aux_body {
		let params = vec![ValType::I64; def.aux_arg_num as usize];
		functions.push((type_index(params, vec![]), body));
	}

	// Grant access to linear memory.
	// Every contract module is required to have an imported memory.
	// If no memory is specified in the passed ModuleDefinition, then
	// default to (1, 1).
	let (init, max) = if let Some(memory) = &def.memory {
		(memory.min_pages, Some(memory.max_pages))
	} else {
		(1, Some(1))
	};
	let mut imports = ImportSection::new();
	imports.import(
		"env",
		"memory",
		MemoryType {
			minimum: init.into(),
			maximum: max.map(Into::into),
			memory64: false,
			shared: false,
			page_size_log2: None,
		},
	);

	// Import supervisor functions. They start with idx 0.
	for func in def.imported_functions {
		let ty = type_index(func.params, func.return_type.into_iter().collect());
		imports.import(func.module, func.name, EntityType::Function(ty));
	}

	let mut module = Module::new();

	let mut type_section = TypeSection::new();
	for (params, results) in &types {
		type_section.ty().function(params.iter().copied(), results.iter().copied());
	}
	module.section(&type_section);
	module.section(&imports);

	let mut function_section = FunctionSection::new();
	for (ty, _) in &functions {
		function_section.function(*ty);
	}
	module.section(&function_section);

	// Add function pointer table
	if let Some(table) = &def.table {
		let mut tables = TableSection::new();
		tables.table(TableType {
			element_type: RefType::FUNCREF,
			table64: false,
			minimum: table.num_elements.into(),
			maximum: Some(table.num_elements.into()),
			shared: false,
		});
		module.section(&tables);
	}

	// Add global variables
	if def.num_globals > 0 {
		use rand::{distributions::Standard, prelude::*};
		let rng = rand_pcg::Pcg32::seed_from_u64(3112244599778833558);
		let mut globals = GlobalSection::new();
		for val in rng.sample_iter(Standard).take(def.num_globals as usize) {
			globals.global(
				GlobalType { val_type: ValType::I64, mutable: true, shared: false },
				&ConstExpr::i64_const(val),
			);
		}
		module.section(&globals);
	}

	let mut exports = ExportSection::new();
	exports.export("deploy", ExportKind::Func, func_offset);
	exports.export("call", ExportKind::Func, func_offset + 1);
	module.section(&exports);

	if let Some(table) = &def.table {
		let mut elements = ElementSection::new();
		let members = vec![table.function_index; table.num_elements as usize];
		elements.active(None, &ConstExpr::i32_const(0), Elements::Functions(members.into()));
		module.section(&elements);
	}

	let mut code = CodeSection::new();
	for (_, body) in &functions {
		let mut function = Function::new(body.locals.iter().copied());
		for instruction in &body.instructions {
			function.instruction(instruction);
		}
		code.function(&function);
	}
	module.section(&code);

	// Initialize memory
	if !def.data_segments.is_empty() {
		let mut data = DataSection::new();
		for segment in def.data_segments {
			data.active(0, &ConstExpr::i32_const(segment.offset as i32), segment.value);
		}
		module.section(&data);
	}

	// Add the dummy section
	if def.dummy_section > 0 {
		module.section(&CustomSection {
			name: "dummy".to_owned().into(),
			data: vec![42; def.dummy_section as usize].into(),
		});
	}

	module.finish()
}

/// Mechanisms to generate a function body that can be used inside a `ModuleDefinition`.
pub mod body {
	use super::*;

	pub fn repeated(repetitions: u32, instructions: &[Instruction<'static>]) -> FuncBody {
		repeated_with_locals(&[], repetitions, instructions)
	}

	pub fn repeated_with_locals(
		locals: &[Local],
		repetitions: u32,
		instructions: &[Instruction<'static>],
	) -> FuncBody {
		let instructions = instructions
			.iter()
			.cycle()
			.take(instructions.len() * usize::try_from(repetitions).unwrap())
			.cloned()
			.chain(core::iter::once(Instruction::End))
			.collect();
		FuncBody { locals: locals.to_vec(), instructions }
	}

	pub fn repeated_with_locals_using<const N: usize>(
		locals: &[Local],
		repetitions: u32,
		mut f: impl FnMut() -> [Instruction<'static>; N],
	) -> FuncBody {
		let mut instructions = Vec::new();
		for _ in 0..repetitions {
			instructions.extend(f());
		}
		instructions.push(Instruction::End);
		FuncBody { locals: locals.to_vec(), instructions }
	}
}
