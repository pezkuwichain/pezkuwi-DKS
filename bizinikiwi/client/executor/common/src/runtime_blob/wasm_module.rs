// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! An owned, section-level view of a WebAssembly module.
//!
//! The executor only ever rewrites a handful of sections (imports, memory, globals, exports,
//! elements, the start function and code), so a module is kept as its list of sections with
//! each payload as raw bytes, and a section is decoded only when it is about to be changed.
//! Sections that are not touched are written back byte for byte.
//!
//! # What is accepted
//!
//! This replaced a `parity-wasm` based implementation that was compiled without any of its
//! post-MVP features, so the executor has only ever accepted WebAssembly MVP modules (plus
//! mutable globals, which `parity-wasm` parses as part of a global type). [`WasmModule::parse`]
//! keeps that boundary: it decodes every section and every instruction, and rejects the module
//! if it uses a post-MVP construct -- sign extension, saturating conversions, bulk memory,
//! reference types, multi-value, SIMD, threads, tail calls, memory64 and so on.
//!
//! Like `parity-wasm`, this is a *syntactic* check: it does not type-check function bodies.
//! Full validation remains the job of the engine that compiles the module.
//!
//! Section layout rules are the ones `parity-wasm` applied: known sections appear at most once
//! and in order (the data count section ordered before code), custom sections may appear
//! anywhere, and the function and code sections must have the same number of entries.

use wasmparser::{
	BinaryReader, BlockType, CodeSectionReader, CompositeInnerType, ConstExpr, DataKind,
	DataSectionReader, ElementItems, ElementKind, ElementSectionReader, ExportSectionReader,
	ExternalKind, FunctionBody, FunctionSectionReader, GlobalSectionReader, GlobalType,
	ImportSectionReader, MemorySectionReader, MemoryType, Operator, RefType, TableInit,
	TableSectionReader, TableType, TypeRef, TypeSectionReader, ValType, WasmFeatures,
};

pub(super) type Result<T> = core::result::Result<T, String>;

/// Section ids, as they appear in the binary encoding.
pub(super) mod id {
	pub const CUSTOM: u8 = 0;
	pub const TYPE: u8 = 1;
	pub const IMPORT: u8 = 2;
	pub const FUNCTION: u8 = 3;
	pub const TABLE: u8 = 4;
	pub const MEMORY: u8 = 5;
	pub const GLOBAL: u8 = 6;
	pub const EXPORT: u8 = 7;
	pub const START: u8 = 8;
	pub const ELEMENT: u8 = 9;
	pub const CODE: u8 = 10;
	pub const DATA: u8 = 11;
	pub const DATA_COUNT: u8 = 12;
}

/// The feature set every reader is created with. Some encodings are read differently once a
/// proposal is enabled (a memory index inside `memarg`, the table byte of `call_indirect`, the
/// memory byte of `memory.size`), and MVP is how the executor has always read them.
fn features() -> WasmFeatures {
	WasmFeatures::WASM1
}

fn reader(bytes: &[u8]) -> BinaryReader<'_> {
	BinaryReader::new_features(bytes, 0, features())
}

fn err(e: impl core::fmt::Display) -> String {
	e.to_string()
}

/// Where a section sits in the required section order. Custom sections can be anywhere and
/// report 0; the data count section (id 12) goes between the element and the code section.
pub(super) fn order(id: u8) -> u8 {
	match id {
		id::DATA_COUNT => 10,
		id::CODE => 11,
		id::DATA => 12,
		other => other,
	}
}

/// One section of a module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Section {
	/// A custom section: its name and its contents after the name.
	Custom { name: String, data: Vec<u8> },
	/// Any other section: its id and its payload, exactly as it appears in the binary.
	Known { id: u8, payload: Vec<u8> },
}

impl Section {
	pub(super) fn id(&self) -> u8 {
		match self {
			Section::Custom { .. } => id::CUSTOM,
			Section::Known { id, .. } => *id,
		}
	}

	fn order(&self) -> u8 {
		order(self.id())
	}
}

/// A WebAssembly module, kept as its list of sections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WasmModule {
	pub(super) sections: Vec<Section>,
}

const MAGIC: [u8; 4] = *b"\0asm";
const VERSION: [u8; 4] = [1, 0, 0, 0];

impl WasmModule {
	/// Decodes a module, rejecting anything outside the subset described in the module docs.
	pub(super) fn parse(bytes: &[u8]) -> Result<Self> {
		if bytes.len() < 4 || bytes[..4] != MAGIC {
			return Err("invalid magic number".into());
		}
		if bytes.len() < 8 || bytes[4..8] != VERSION {
			return Err("unsupported version".into());
		}

		let mut input = reader(&bytes[8..]);
		let mut sections = Vec::new();
		let mut last_order = 0;
		let mut function_count = 0;
		let mut code_count = 0;
		while !input.eof() {
			let section_id = input.read_u8().map_err(err)?;
			let size = input.read_var_u32().map_err(err)?;
			let payload = input.read_bytes(size as usize).map_err(err)?;

			if section_id == id::CUSTOM {
				let mut custom = reader(payload);
				// No length cap on the name: the previous decoder had none either.
				let name = custom.read_unlimited_string().map_err(err)?.to_owned();
				let data = custom.read_bytes(custom.bytes_remaining()).map_err(err)?.to_vec();
				sections.push(Section::Custom { name, data });
				continue;
			}
			if section_id > id::DATA_COUNT {
				return Err(format!("invalid section id {section_id}"));
			}
			let section_order = order(section_id);
			if last_order > section_order {
				return Err("sections out of order".into());
			}
			if last_order == section_order {
				return Err(format!("duplicated section {section_id}"));
			}
			last_order = section_order;

			let count = check_section(section_id, payload)?;
			match section_id {
				id::FUNCTION => function_count = count,
				id::CODE => code_count = count,
				_ => {},
			}
			sections.push(Section::Known { id: section_id, payload: payload.to_vec() });
		}

		if function_count != code_count {
			return Err("function and code section have inconsistent lengths".into());
		}
		Ok(Self { sections })
	}

	/// Encodes the module.
	pub(super) fn serialize(&self) -> Vec<u8> {
		let mut out = Vec::with_capacity(
			8 + self
				.sections
				.iter()
				.map(|s| match s {
					Section::Custom { name, data } => name.len() + data.len() + 16,
					Section::Known { payload, .. } => payload.len() + 6,
				})
				.sum::<usize>(),
		);
		out.extend_from_slice(&MAGIC);
		out.extend_from_slice(&VERSION);
		for section in &self.sections {
			out.push(section.id());
			match section {
				Section::Custom { name, data } => {
					let mut payload = Vec::with_capacity(name.len() + data.len() + 5);
					write_name(&mut payload, name);
					payload.extend_from_slice(data);
					write_u32(&mut out, payload.len() as u32);
					out.extend_from_slice(&payload);
				},
				Section::Known { payload, .. } => {
					write_u32(&mut out, payload.len() as u32);
					out.extend_from_slice(payload);
				},
			}
		}
		out
	}

	/// The payload of the first section with the given (non-custom) id.
	pub(super) fn known(&self, section_id: u8) -> Option<&[u8]> {
		self.sections.iter().find_map(|s| match s {
			Section::Known { id, payload } if *id == section_id => Some(&payload[..]),
			_ => None,
		})
	}

	/// The payload of the first section with the given (non-custom) id, mutably.
	pub(super) fn known_mut(&mut self, section_id: u8) -> Option<&mut Vec<u8>> {
		self.sections.iter_mut().find_map(|s| match s {
			Section::Known { id, payload } if *id == section_id => Some(payload),
			_ => None,
		})
	}

	/// The contents of the first custom section with the given name.
	pub(super) fn custom_section(&self, section_name: &str) -> Option<&[u8]> {
		self.sections.iter().find_map(|s| match s {
			Section::Custom { name, data } if name == section_name => Some(&data[..]),
			_ => None,
		})
	}

	/// Inserts a non-custom section where the section order puts it: before the first
	/// section that must come after it. Fails if a section with that id already exists.
	pub(super) fn insert_section(&mut self, section_id: u8, payload: Vec<u8>) -> Result<()> {
		if self.known(section_id).is_some() {
			return Err(format!("duplicated section {section_id}"));
		}
		let section = Section::Known { id: section_id, payload };
		match self.sections.iter().position(|s| section.order() < s.order()) {
			Some(pos) => self.sections.insert(pos, section),
			None => self.sections.push(section),
		}
		Ok(())
	}

	/// Removes the first memory import, returning its field name and limits. The import section
	/// stays, even if it is left empty.
	pub(super) fn take_memory_import(&mut self) -> Result<Option<(String, Limits)>> {
		let Some(payload) = self.known_mut(id::IMPORT) else { return Ok(None) };
		let found = {
			let imports = ImportSectionReader::new(reader(payload)).map_err(err)?;
			let mut items = imports.into_iter_with_offsets().peekable();
			let mut found = None;
			while let Some(item) = items.next() {
				let (start, import) = item.map_err(err)?;
				let TypeRef::Memory(memory) = import.ty else { continue };
				let end = match items.peek() {
					Some(Ok((next, _))) => *next,
					Some(Err(e)) => return Err(err(e)),
					None => payload.len(),
				};
				found = Some((start, end, import.name.to_owned(), Limits::from_memory(&memory)));
				break;
			}
			found
		};
		let Some((start, end, name, limits)) = found else { return Ok(None) };

		let (count, entries) = split_count(payload)?;
		let entries_start = payload.len() - entries.len();
		let mut rest = Vec::with_capacity(payload.len());
		write_u32(&mut rest, count - 1);
		rest.extend_from_slice(&payload[entries_start..start]);
		rest.extend_from_slice(&payload[end..]);
		*payload = rest;
		Ok(Some((name, limits)))
	}

	/// The type index of every imported function, in import order.
	pub(super) fn imported_function_types(&self) -> Result<Vec<u32>> {
		let Some(payload) = self.known(id::IMPORT) else { return Ok(Vec::new()) };
		let mut types = Vec::new();
		for import in ImportSectionReader::new(reader(payload)).map_err(err)? {
			if let TypeRef::Func(ty) = import.map_err(err)?.ty {
				types.push(ty);
			}
		}
		Ok(types)
	}

	/// The type index of every function defined in the module.
	pub(super) fn defined_function_types(&self) -> Result<Vec<u32>> {
		let Some(payload) = self.known(id::FUNCTION) else { return Ok(Vec::new()) };
		FunctionSectionReader::new(reader(payload))
			.map_err(err)?
			.into_iter()
			.map(|t| t.map_err(err))
			.collect()
	}

	/// Every type in the type section.
	pub(super) fn types(&self) -> Result<Vec<Signature>> {
		let Some(payload) = self.known(id::TYPE) else { return Ok(Vec::new()) };
		let mut types = Vec::new();
		for group in TypeSectionReader::new(reader(payload)).map_err(err)? {
			for sub_type in group.map_err(err)?.into_types() {
				let ty = sub_type.unwrap_func();
				types.push(Signature {
					params: ty.params().to_vec(),
					results: ty.results().to_vec(),
				});
			}
		}
		Ok(types)
	}

	/// The raw bytes of every function body, without the size prefix.
	pub(super) fn function_bodies(&self) -> Result<Vec<&[u8]>> {
		let Some(payload) = self.known(id::CODE) else { return Ok(Vec::new()) };
		CodeSectionReader::new(reader(payload))
			.map_err(err)?
			.into_iter()
			.map(|body| body.map(|b| b.as_bytes()).map_err(err))
			.collect()
	}

	/// The limits of every memory defined in the memory section, if there is one.
	pub(super) fn memories(&self) -> Result<Option<Vec<Limits>>> {
		let Some(payload) = self.known(id::MEMORY) else { return Ok(None) };
		MemorySectionReader::new(reader(payload))
			.map_err(err)?
			.into_iter()
			.map(|m| m.map(|m| Limits::from_memory(&m)).map_err(err))
			.collect::<Result<Vec<_>>>()
			.map(Some)
	}

	/// Replaces the memory section's entries.
	pub(super) fn set_memories(&mut self, memories: &[Limits]) {
		let payload = encode_vec(memories, |out, m| m.encode(out));
		*self
			.known_mut(id::MEMORY)
			.expect("only called when a memory section exists; qed") = payload;
	}
}

/// A function signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Signature {
	pub params: Vec<ValType>,
	pub results: Vec<ValType>,
}

/// Memory limits, in pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Limits {
	pub initial: u32,
	pub maximum: Option<u32>,
}

impl Limits {
	pub(super) fn from_memory(memory: &MemoryType) -> Self {
		// `parse` only admits 32-bit memories, whose limits are read as `u32`.
		Self { initial: memory.initial as u32, maximum: memory.maximum.map(|m| m as u32) }
	}

	pub(super) fn encode(&self, out: &mut Vec<u8>) {
		match self.maximum {
			None => {
				out.push(0x00);
				write_u32(out, self.initial);
			},
			Some(maximum) => {
				out.push(0x01);
				write_u32(out, self.initial);
				write_u32(out, maximum);
			},
		}
	}
}

/// Writes `value` as an unsigned LEB128.
pub(super) fn write_u32(out: &mut Vec<u8>, mut value: u32) {
	loop {
		let byte = (value & 0x7f) as u8;
		value >>= 7;
		if value == 0 {
			out.push(byte);
			return;
		}
		out.push(byte | 0x80);
	}
}

/// Writes `value` as a signed LEB128.
pub(super) fn write_i32(out: &mut Vec<u8>, mut value: i32) {
	loop {
		let byte = (value & 0x7f) as u8;
		value >>= 7;
		let done = (value == 0 && byte & 0x40 == 0) || (value == -1 && byte & 0x40 != 0);
		if done {
			out.push(byte);
			return;
		}
		out.push(byte | 0x80);
	}
}

/// Writes a length-prefixed UTF-8 name.
pub(super) fn write_name(out: &mut Vec<u8>, name: &str) {
	write_u32(out, name.len() as u32);
	out.extend_from_slice(name.as_bytes());
}

/// Encodes a vector: its length, then each item.
pub(super) fn encode_vec<T>(items: &[T], mut each: impl FnMut(&mut Vec<u8>, &T)) -> Vec<u8> {
	let mut out = Vec::new();
	write_u32(&mut out, items.len() as u32);
	for item in items {
		each(&mut out, item);
	}
	out
}

/// Splits a vector-shaped section payload into its item count and the raw items.
pub(super) fn split_count(payload: &[u8]) -> Result<(u32, &[u8])> {
	let mut r = reader(payload);
	let count = r.read_var_u32().map_err(err)?;
	Ok((count, &payload[r.current_position()..]))
}

/// Appends raw, already encoded items to a vector-shaped section payload.
pub(super) fn append_items(payload: &[u8], added: u32, items: &[u8]) -> Result<Vec<u8>> {
	let (count, existing) = split_count(payload)?;
	let count = count.checked_add(added).ok_or("section item count overflow")?;
	let mut out = Vec::with_capacity(payload.len() + items.len() + 5);
	write_u32(&mut out, count);
	out.extend_from_slice(existing);
	out.extend_from_slice(items);
	Ok(out)
}

/// An export entry.
pub(super) struct Export {
	pub name: String,
	pub kind: ExternalKind,
	pub index: u32,
}

impl Export {
	pub(super) fn encode(&self, out: &mut Vec<u8>) {
		write_name(out, &self.name);
		out.push(match self.kind {
			ExternalKind::Func => 0x00,
			ExternalKind::Table => 0x01,
			ExternalKind::Memory => 0x02,
			ExternalKind::Global => 0x03,
			ExternalKind::Tag => 0x04,
		});
		write_u32(out, self.index);
	}
}

pub(super) fn exports(payload: &[u8]) -> Result<Vec<Export>> {
	ExportSectionReader::new(reader(payload))
		.map_err(err)?
		.into_iter()
		.map(|e| {
			e.map(|e| Export { name: e.name.to_owned(), kind: e.kind, index: e.index })
				.map_err(err)
		})
		.collect()
}

/// An active element segment for table 0.
pub(super) struct ElementSegment {
	/// The offset expression, including its terminating `end`.
	pub offset: Vec<u8>,
	pub members: Vec<u32>,
}

impl ElementSegment {
	pub(super) fn encode(&self, out: &mut Vec<u8>) {
		// Flags 0: active, table 0, function indices.
		out.push(0x00);
		out.extend_from_slice(&self.offset);
		write_u32(out, self.members.len() as u32);
		for member in &self.members {
			write_u32(out, *member);
		}
	}
}

pub(super) fn element_segments(payload: &[u8]) -> Result<Vec<ElementSegment>> {
	let mut segments = Vec::new();
	for element in ElementSectionReader::new(reader(payload)).map_err(err)? {
		let element = element.map_err(err)?;
		let (
			ElementKind::Active { table_index: None, offset_expr },
			ElementItems::Functions(items),
		) = (element.kind, element.items)
		else {
			return Err("unsupported element segment".into());
		};
		let members = items.into_iter().map(|m| m.map_err(err)).collect::<Result<Vec<_>>>()?;
		segments.push(ElementSegment { offset: const_expr_bytes(&offset_expr)?, members });
	}
	Ok(segments)
}

pub(super) fn start_function(payload: &[u8]) -> Result<u32> {
	reader(payload).read_var_u32().map_err(err)
}

fn const_expr_bytes(expr: &ConstExpr) -> Result<Vec<u8>> {
	let mut r = expr.get_binary_reader();
	Ok(r.read_bytes(r.bytes_remaining()).map_err(err)?.to_vec())
}

/// Checks one non-custom section and returns its item count.
fn check_section(section_id: u8, payload: &[u8]) -> Result<u32> {
	let r = reader(payload);
	match section_id {
		id::TYPE => {
			let types = TypeSectionReader::new(r).map_err(err)?;
			let count = types.count();
			for item in types.into_iter_with_offsets() {
				let (offset, group) = item.map_err(err)?;
				// Only the plain `func` form (0x60); no recursion groups, sub types or GC types.
				if payload[offset] != 0x60 || group.is_explicit_rec_group() {
					return Err("unsupported type definition".into());
				}
				for sub_type in group.into_types() {
					let CompositeInnerType::Func(ty) = &sub_type.composite_type.inner else {
						return Err("unsupported type definition".into());
					};
					if !sub_type.is_final
						|| sub_type.supertype_idx.is_some()
						|| sub_type.composite_type.shared
					{
						return Err("unsupported type definition".into());
					}
					if ty.results().len() > 1 {
						return Err("more than one function result".into());
					}
					ty.params().iter().chain(ty.results()).try_for_each(check_val_type)?;
				}
			}
			Ok(count)
		},
		id::IMPORT => {
			let imports = ImportSectionReader::new(r).map_err(err)?;
			let count = imports.count();
			for import in imports {
				match import.map_err(err)?.ty {
					TypeRef::Func(_) => {},
					TypeRef::Table(ty) => check_table_type(&ty)?,
					TypeRef::Memory(ty) => check_memory_type(&ty)?,
					TypeRef::Global(ty) => check_global_type(&ty)?,
					TypeRef::Tag(_) => return Err("unsupported import kind".into()),
				}
			}
			Ok(count)
		},
		id::FUNCTION => {
			let functions = FunctionSectionReader::new(r).map_err(err)?;
			let count = functions.count();
			for function in functions {
				function.map_err(err)?;
			}
			Ok(count)
		},
		id::TABLE => {
			let tables = TableSectionReader::new(r).map_err(err)?;
			let count = tables.count();
			for table in tables {
				let table = table.map_err(err)?;
				if !matches!(table.init, TableInit::RefNull) {
					return Err("unsupported table initializer".into());
				}
				check_table_type(&table.ty)?;
			}
			Ok(count)
		},
		id::MEMORY => {
			let memories = MemorySectionReader::new(r).map_err(err)?;
			let count = memories.count();
			for memory in memories {
				check_memory_type(&memory.map_err(err)?)?;
			}
			Ok(count)
		},
		id::GLOBAL => {
			let globals = GlobalSectionReader::new(r).map_err(err)?;
			let count = globals.count();
			for global in globals {
				let global = global.map_err(err)?;
				check_global_type(&global.ty)?;
				check_const_expr(&global.init_expr)?;
			}
			Ok(count)
		},
		id::EXPORT => {
			let exports = ExportSectionReader::new(r).map_err(err)?;
			let count = exports.count();
			for export in exports {
				if export.map_err(err)?.kind == ExternalKind::Tag {
					return Err("unsupported export kind".into());
				}
			}
			Ok(count)
		},
		id::START | id::DATA_COUNT => {
			let mut r = r;
			r.read_var_u32().map_err(err)?;
			if !r.eof() {
				return Err("trailing bytes in section".into());
			}
			Ok(1)
		},
		id::ELEMENT => {
			let elements = ElementSectionReader::new(r).map_err(err)?;
			let count = elements.count();
			for element in elements {
				let element = element.map_err(err)?;
				let ElementKind::Active { table_index: None, offset_expr } = element.kind else {
					return Err("unsupported element segment".into());
				};
				let ElementItems::Functions(items) = element.items else {
					return Err("unsupported element segment".into());
				};
				check_const_expr(&offset_expr)?;
				for item in items {
					item.map_err(err)?;
				}
			}
			Ok(count)
		},
		id::CODE => {
			let bodies = CodeSectionReader::new(r).map_err(err)?;
			let count = bodies.count();
			for body in bodies {
				decode_body(&body.map_err(err)?)?;
			}
			Ok(count)
		},
		id::DATA => {
			let data = DataSectionReader::new(r).map_err(err)?;
			let count = data.count();
			for item in data.into_iter_with_offsets() {
				let (offset, segment) = item.map_err(err)?;
				// The leading field is the memory index in MVP (`0`); a non-zero value means a
				// bulk-memory form, even when it then names memory 0.
				let flags = reader(&payload[offset..]).read_var_u32().map_err(err)?;
				let DataKind::Active { memory_index: 0, offset_expr } = segment.kind else {
					return Err("unsupported data segment".into());
				};
				if flags != 0 {
					return Err("unsupported data segment".into());
				}
				check_const_expr(&offset_expr)?;
			}
			Ok(count)
		},
		other => Err(format!("invalid section id {other}")),
	}
}

fn check_val_type(ty: &ValType) -> Result<()> {
	match ty {
		ValType::I32 | ValType::I64 | ValType::F32 | ValType::F64 => Ok(()),
		_ => Err("unsupported value type".into()),
	}
}

fn check_table_type(ty: &TableType) -> Result<()> {
	if ty.element_type != RefType::FUNCREF || ty.table64 || ty.shared {
		return Err("unsupported table type".into());
	}
	Ok(())
}

fn check_memory_type(ty: &MemoryType) -> Result<()> {
	if ty.memory64 || ty.shared || ty.page_size_log2.is_some() {
		return Err("unsupported memory type".into());
	}
	Ok(())
}

fn check_global_type(ty: &GlobalType) -> Result<()> {
	if ty.shared {
		return Err("unsupported global type".into());
	}
	check_val_type(&ty.content_type)
}

fn check_const_expr(expr: &ConstExpr) -> Result<()> {
	let mut ops = expr.get_operators_reader();
	while !ops.eof() {
		classify(&ops.read().map_err(err)?).ok_or("unsupported instruction")?;
	}
	ops.finish().map_err(err)
}

/// How an instruction moves the value stack. `Pop` and `Push` are applied in order: the
/// stack-height pass treats `pop 2, pop 1` differently from `pop 3` once a frame is
/// unreachable, so the steps are kept exactly as the original pass applied them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
	Pop(u32),
	Push(u32),
}

const NONE: &[Step] = &[];
const PUSH: &[Step] = &[Step::Push(1)];
const POP: &[Step] = &[Step::Pop(1)];
const POP_PUSH: &[Step] = &[Step::Pop(1), Step::Push(1)];
const POP2: &[Step] = &[Step::Pop(2)];
const POP2_PUSH: &[Step] = &[Step::Pop(2), Step::Push(1)];
const SELECT: &[Step] = &[Step::Pop(2), Step::Pop(1), Step::Push(1)];

/// An MVP instruction, reduced to what the stack-height pass needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Instr {
	Block { results: u32 },
	Loop { results: u32 },
	If { results: u32 },
	Else,
	End,
	Unreachable,
	Br(u32),
	BrIf(u32),
	BrTable { targets: Vec<u32>, default: u32 },
	Return,
	Call(u32),
	CallIndirect(u32),
	Plain(&'static [Step]),
}

fn block_results(ty: &BlockType) -> Option<u32> {
	match ty {
		BlockType::Empty => Some(0),
		BlockType::Type(ty) => check_val_type(ty).ok().map(|()| 1),
		BlockType::FuncType(_) => None,
	}
}

/// Maps an operator to its [`Instr`], or `None` if it is not an MVP instruction.
pub(super) fn classify(op: &Operator) -> Option<Instr> {
	use Operator as O;
	Some(match op {
		O::Nop => Instr::Plain(NONE),
		O::Block { blockty } => Instr::Block { results: block_results(blockty)? },
		O::Loop { blockty } => Instr::Loop { results: block_results(blockty)? },
		O::If { blockty } => Instr::If { results: block_results(blockty)? },
		O::Else => Instr::Else,
		O::End => Instr::End,
		O::Unreachable => Instr::Unreachable,
		O::Br { relative_depth } => Instr::Br(*relative_depth),
		O::BrIf { relative_depth } => Instr::BrIf(*relative_depth),
		O::BrTable { targets } => Instr::BrTable {
			targets: targets.targets().collect::<core::result::Result<_, _>>().ok()?,
			default: targets.default(),
		},
		O::Return => Instr::Return,
		O::Call { function_index } => Instr::Call(*function_index),
		O::CallIndirect { type_index, table_index: 0 } => Instr::CallIndirect(*type_index),
		O::Drop => Instr::Plain(POP),
		O::Select => Instr::Plain(SELECT),
		O::LocalGet { .. } | O::GlobalGet { .. } => Instr::Plain(PUSH),
		O::LocalSet { .. } | O::GlobalSet { .. } => Instr::Plain(POP),
		O::LocalTee { .. } => Instr::Plain(POP_PUSH),

		O::I32Load { memarg }
		| O::I64Load { memarg }
		| O::F32Load { memarg }
		| O::F64Load { memarg }
		| O::I32Load8S { memarg }
		| O::I32Load8U { memarg }
		| O::I32Load16S { memarg }
		| O::I32Load16U { memarg }
		| O::I64Load8S { memarg }
		| O::I64Load8U { memarg }
		| O::I64Load16S { memarg }
		| O::I64Load16U { memarg }
		| O::I64Load32S { memarg }
		| O::I64Load32U { memarg }
			if memarg.memory == 0 =>
		{
			Instr::Plain(POP_PUSH)
		},

		O::I32Store { memarg }
		| O::I64Store { memarg }
		| O::F32Store { memarg }
		| O::F64Store { memarg }
		| O::I32Store8 { memarg }
		| O::I32Store16 { memarg }
		| O::I64Store8 { memarg }
		| O::I64Store16 { memarg }
		| O::I64Store32 { memarg }
			if memarg.memory == 0 =>
		{
			Instr::Plain(POP2)
		},

		O::MemorySize { mem: 0 } => Instr::Plain(PUSH),
		O::MemoryGrow { mem: 0 } => Instr::Plain(POP_PUSH),

		O::I32Const { .. } | O::I64Const { .. } | O::F32Const { .. } | O::F64Const { .. } => {
			Instr::Plain(PUSH)
		},

		O::I32Eqz | O::I64Eqz => Instr::Plain(POP_PUSH),

		O::I32Eq
		| O::I32Ne
		| O::I32LtS
		| O::I32LtU
		| O::I32GtS
		| O::I32GtU
		| O::I32LeS
		| O::I32LeU
		| O::I32GeS
		| O::I32GeU
		| O::I64Eq
		| O::I64Ne
		| O::I64LtS
		| O::I64LtU
		| O::I64GtS
		| O::I64GtU
		| O::I64LeS
		| O::I64LeU
		| O::I64GeS
		| O::I64GeU
		| O::F32Eq
		| O::F32Ne
		| O::F32Lt
		| O::F32Gt
		| O::F32Le
		| O::F32Ge
		| O::F64Eq
		| O::F64Ne
		| O::F64Lt
		| O::F64Gt
		| O::F64Le
		| O::F64Ge => Instr::Plain(POP2_PUSH),

		O::I32Clz
		| O::I32Ctz
		| O::I32Popcnt
		| O::I64Clz
		| O::I64Ctz
		| O::I64Popcnt
		| O::F32Abs
		| O::F32Neg
		| O::F32Ceil
		| O::F32Floor
		| O::F32Trunc
		| O::F32Nearest
		| O::F32Sqrt
		| O::F64Abs
		| O::F64Neg
		| O::F64Ceil
		| O::F64Floor
		| O::F64Trunc
		| O::F64Nearest
		| O::F64Sqrt => Instr::Plain(POP_PUSH),

		O::I32Add
		| O::I32Sub
		| O::I32Mul
		| O::I32DivS
		| O::I32DivU
		| O::I32RemS
		| O::I32RemU
		| O::I32And
		| O::I32Or
		| O::I32Xor
		| O::I32Shl
		| O::I32ShrS
		| O::I32ShrU
		| O::I32Rotl
		| O::I32Rotr
		| O::I64Add
		| O::I64Sub
		| O::I64Mul
		| O::I64DivS
		| O::I64DivU
		| O::I64RemS
		| O::I64RemU
		| O::I64And
		| O::I64Or
		| O::I64Xor
		| O::I64Shl
		| O::I64ShrS
		| O::I64ShrU
		| O::I64Rotl
		| O::I64Rotr
		| O::F32Add
		| O::F32Sub
		| O::F32Mul
		| O::F32Div
		| O::F32Min
		| O::F32Max
		| O::F32Copysign
		| O::F64Add
		| O::F64Sub
		| O::F64Mul
		| O::F64Div
		| O::F64Min
		| O::F64Max
		| O::F64Copysign => Instr::Plain(POP2_PUSH),

		O::I32WrapI64
		| O::I32TruncF32S
		| O::I32TruncF32U
		| O::I32TruncF64S
		| O::I32TruncF64U
		| O::I64ExtendI32S
		| O::I64ExtendI32U
		| O::I64TruncF32S
		| O::I64TruncF32U
		| O::I64TruncF64S
		| O::I64TruncF64U
		| O::F32ConvertI32S
		| O::F32ConvertI32U
		| O::F32ConvertI64S
		| O::F32ConvertI64U
		| O::F32DemoteF64
		| O::F64ConvertI32S
		| O::F64ConvertI32U
		| O::F64ConvertI64S
		| O::F64ConvertI64U
		| O::F64PromoteF32
		| O::I32ReinterpretF32
		| O::I64ReinterpretF64
		| O::F32ReinterpretI32
		| O::F64ReinterpretI64 => Instr::Plain(POP_PUSH),

		_ => return None,
	})
}

/// A decoded function body.
pub(super) struct DecodedBody {
	/// The number of locals the body declares (not counting parameters).
	pub locals: u32,
	/// Every instruction with the offset of its first byte in the body bytes.
	pub instrs: Vec<(Instr, usize)>,
}

/// Decodes a function body, rejecting anything but MVP locals and instructions and any bytes
/// after the `end` that closes the function.
pub(super) fn decode_body(body: &FunctionBody) -> Result<DecodedBody> {
	let base = body.range().start;
	let mut locals_reader = body.get_locals_reader().map_err(err)?;
	let mut locals: u32 = 0;
	for _ in 0..locals_reader.get_count() {
		let (count, ty) = locals_reader.read().map_err(err)?;
		check_val_type(&ty)?;
		locals = locals.checked_add(count).ok_or("too many locals")?;
	}
	let mut ops = body.get_operators_reader().map_err(err)?;
	let mut instrs = Vec::new();
	let mut depth: u32 = 1;
	while !ops.eof() {
		if depth == 0 {
			return Err("trailing bytes after the end of a function body".into());
		}
		let (op, offset) = ops.read_with_offset().map_err(err)?;
		let instr = classify(&op).ok_or("unsupported instruction")?;
		match instr {
			Instr::Block { .. } | Instr::Loop { .. } | Instr::If { .. } => depth += 1,
			Instr::End => depth -= 1,
			_ => {},
		}
		instrs.push((instr, offset - base));
	}
	if depth != 0 {
		return Err("function body is not terminated".into());
	}
	Ok(DecodedBody { locals, instrs })
}

/// Decodes a function body given as raw bytes (without the size prefix).
pub(super) fn decode_body_bytes(bytes: &[u8]) -> Result<DecodedBody> {
	decode_body(&FunctionBody::new(reader(bytes)))
}
