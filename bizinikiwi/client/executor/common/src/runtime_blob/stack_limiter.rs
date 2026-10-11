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

//! Deterministic stack-height metering.
//!
//! This is a port of the stack limiter of `wasm-instrument` 0.4 (which was built on the
//! deprecated `parity-wasm`). The pass decides when a teyrchain validation function traps with
//! a stack overflow, so validators must agree on its output: it is reproduced instruction for
//! instruction, quirks included, and pinned by recorded vectors of the original in
//! `tests/vectors`.
//!
//! The pass adds a mutable `i32` global holding the current stack height and wraps every call
//! to a defined function:
//!
//! ```text
//! global.get $h  i32.const COST  i32.add  global.set $h
//! global.get $h  i32.const LIMIT i32.gt_u  if  unreachable  end
//! call $f
//! global.get $h  i32.const COST  i32.sub  global.set $h
//! ```
//!
//! where `COST` is the callee's locals (parameters included) plus the deepest its value stack
//! can get, plus an activation-frame cost of 2. Functions entered from outside -- exports,
//! table entries and the start function -- are reached through generated "thunks" that run
//! the same sequence around the original function.
//!
//! The quirks that are kept on purpose:
//! - the new global's index is the number of *defined* globals minus one, ignoring imported
//!   globals (a module that imports globals has the counter aliased onto another global);
//! - `else` does not reset the value stack height of the `if` frame;
//! - popping from a frame that became unreachable is a no-op only when the height is exactly at
//!   the frame's start;
//! - the module is reassembled the way `parity-wasm`'s builder did it: known sections in
//!   canonical order with empty ones dropped, then every custom section (and a data count
//!   section, if any) in reverse order of appearance.

use super::wasm_module::{
	self as wm, element_segments, exports, id, start_function, write_i32, write_u32, Instr, Result,
	Section, Signature, Step, WasmModule,
};
use std::collections::BTreeMap;
use wasmparser::{ExternalKind, ValType};

/// Stack items charged to every call, for the context the host keeps per activation.
const ACTIVATION_FRAME_COST: u32 = 2;

struct Context {
	types: Vec<Signature>,
	/// Type index of every imported function.
	imported: Vec<u32>,
	/// Type index of every defined function.
	defined: Vec<u32>,
}

impl Context {
	fn resolve_func_type(&self, func_idx: u32) -> Result<&Signature> {
		let sig_idx = match (func_idx as usize).checked_sub(self.imported.len()) {
			None => self.imported[func_idx as usize],
			Some(defined_idx) => *self
				.defined
				.get(defined_idx)
				.ok_or("Function at the specified index is not defined")?,
		};
		Ok(self
			.types
			.get(sig_idx as usize)
			.ok_or("The signature as specified by a function isn't defined")?)
	}
}

/// Instruments `module` so that its stack height cannot exceed `stack_limit`.
pub(super) fn inject(mut module: WasmModule, stack_limit: u32) -> Result<WasmModule> {
	let global_idx = generate_stack_height_global(&mut module)?;

	let ctx = Context {
		types: module.types()?,
		imported: module.imported_function_types()?,
		defined: module.defined_function_types()?,
	};
	let raw_bodies = module.function_bodies()?;
	let bodies = raw_bodies
		.iter()
		.map(|b| wm::decode_body_bytes(b))
		.collect::<Result<Vec<_>>>()?;

	// Stack cost of every function in the function index space; imports cost nothing.
	let mut costs = vec![0; ctx.imported.len()];
	for defined_idx in 0..ctx.defined.len() {
		let body = bodies.get(defined_idx).ok_or("Function body is out of bounds")?;
		let max_height = max_height(&ctx, defined_idx, body)?;
		costs.push(
			body.locals
				.checked_add(max_height)
				.ok_or("Overflow in adding locals_count and max_stack_height")?,
		);
	}

	// Wrap every call to a function with a non-zero cost.
	let mut code = Vec::new();
	let mut instrumented = false;
	for (raw, body) in raw_bodies.iter().zip(&bodies) {
		let new_body = instrument_body(raw, body, &costs, global_idx, stack_limit);
		instrumented |= new_body.is_some();
		let new_body = new_body.as_deref().unwrap_or(raw);
		write_u32(&mut code, new_body.len() as u32);
		code.extend_from_slice(new_body);
	}
	if instrumented {
		let payload = module.known_mut(id::CODE).expect("a call was instrumented; qed");
		*payload = wm::append_items(&[0], bodies.len() as u32, &code)?;
	}

	generate_thunks(module, &ctx, &costs, global_idx, stack_limit)
}

/// Appends the stack-height counter to the global section, creating the section at the end of
/// the module if there is none, and returns its index.
fn generate_stack_height_global(module: &mut WasmModule) -> Result<u32> {
	// mutable i32, initialised with `i32.const 0`
	const ENTRY: [u8; 5] = [0x7f, 0x01, 0x41, 0x00, 0x0b];
	if let Some(payload) = module.known_mut(id::GLOBAL) {
		let (count, _) = wm::split_count(payload)?;
		*payload = wm::append_items(payload, 1, &ENTRY)?;
		// The index among *defined* globals; see the module docs.
		return Ok(count);
	}
	let mut payload = vec![1];
	payload.extend_from_slice(&ENTRY);
	module.sections.push(Section::Known { id: id::GLOBAL, payload });
	Ok(0)
}

/// The value-stack bookkeeping of one control frame.
struct Frame {
	/// Set once an instruction that never falls through has been seen in the frame.
	is_polymorphic: bool,
	/// Values pushed when the frame ends.
	end_arity: u32,
	/// Values popped by a branch to the frame (none for a loop).
	branch_arity: u32,
	/// Value stack height on entry.
	start_height: u32,
}

struct Stack {
	height: u32,
	control: Vec<Frame>,
}

impl Stack {
	fn frame(&self, rel_depth: u32) -> Result<&Frame> {
		let last = self.control.len().checked_sub(1).ok_or("control stack is empty")?;
		let idx = last.checked_sub(rel_depth as usize).ok_or("control stack out-of-bounds")?;
		Ok(&self.control[idx])
	}

	fn mark_unreachable(&mut self) -> Result<()> {
		self.control.last_mut().ok_or("stack must be non-empty")?.is_polymorphic = true;
		Ok(())
	}

	fn push_values(&mut self, count: u32) -> Result<()> {
		self.height = self.height.checked_add(count).ok_or("stack overflow")?;
		Ok(())
	}

	fn pop_values(&mut self, count: u32) -> Result<()> {
		if count == 0 {
			return Ok(());
		}
		let top = self.frame(0)?;
		if self.height == top.start_height {
			// Popping below the frame is an error unless the frame is unreachable, in which
			// case the height stays where it is.
			return if top.is_polymorphic {
				Ok(())
			} else {
				Err("trying to pop more values than pushed".into())
			};
		}
		self.height = self.height.checked_sub(count).ok_or("stack underflow")?;
		Ok(())
	}

	fn apply(&mut self, steps: &[Step]) -> Result<()> {
		for step in steps {
			match *step {
				Step::Pop(n) => self.pop_values(n)?,
				Step::Push(n) => self.push_values(n)?,
			}
		}
		Ok(())
	}
}

/// The deepest the value stack of a defined function can get, plus the activation frame.
fn max_height(ctx: &Context, defined_idx: usize, body: &wm::DecodedBody) -> Result<u32> {
	let sig_idx = ctx.defined[defined_idx];
	let signature =
		ctx.types.get(sig_idx as usize).ok_or("Function is not found in func section")?;
	let func_arity = signature.results.len() as u32;

	let mut stack = Stack { height: ACTIVATION_FRAME_COST, control: Vec::new() };
	stack.control.push(Frame {
		is_polymorphic: false,
		end_arity: func_arity,
		branch_arity: func_arity,
		start_height: 0,
	});
	let mut max_height: u32 = 0;

	for (instr, _) in &body.instrs {
		// Unreachable code does not raise the maximum.
		if stack.height > max_height && !stack.frame(0)?.is_polymorphic {
			max_height = stack.height;
		}

		match instr {
			Instr::Block { results } | Instr::Loop { results } | Instr::If { results } => {
				let end_arity = *results;
				let branch_arity = if matches!(instr, Instr::Loop { .. }) { 0 } else { end_arity };
				if matches!(instr, Instr::If { .. }) {
					stack.pop_values(1)?;
				}
				let start_height = stack.height;
				stack.control.push(Frame {
					is_polymorphic: false,
					end_arity,
					branch_arity,
					start_height,
				});
			},
			// The frame pushed by `if` stays as it is.
			Instr::Else => {},
			Instr::End => {
				let frame = stack.control.pop().ok_or("stack must be non-empty")?;
				stack.height = frame.start_height;
				stack.push_values(frame.end_arity)?;
			},
			Instr::Unreachable => stack.mark_unreachable()?,
			Instr::Br(target) => {
				let arity = stack.frame(*target)?.branch_arity;
				stack.pop_values(arity)?;
				stack.mark_unreachable()?;
			},
			Instr::BrIf(target) => {
				let arity = stack.frame(*target)?.branch_arity;
				stack.pop_values(arity)?;
				stack.pop_values(1)?;
				stack.push_values(arity)?;
			},
			Instr::BrTable { targets, default } => {
				let arity = stack.frame(*default)?.branch_arity;
				for target in targets {
					if stack.frame(*target)?.branch_arity != arity {
						return Err("Arity of all jump-targets must be equal".into());
					}
				}
				stack.pop_values(arity)?;
				stack.mark_unreachable()?;
			},
			Instr::Return => {
				stack.pop_values(func_arity)?;
				stack.mark_unreachable()?;
			},
			Instr::Call(func_idx) => {
				let ty = ctx.resolve_func_type(*func_idx)?;
				stack.pop_values(ty.params.len() as u32)?;
				stack.push_values(ty.results.len() as u32)?;
			},
			Instr::CallIndirect(type_idx) => {
				let ty = ctx.types.get(*type_idx as usize).ok_or("Type not found")?;
				stack.pop_values(1)?;
				stack.pop_values(ty.params.len() as u32)?;
				stack.push_values(ty.results.len() as u32)?;
			},
			Instr::Plain(steps) => stack.apply(steps)?,
		}
	}

	Ok(max_height)
}

/// Appends the instructions that surround a call: the preamble before it, or the postamble
/// after it.
fn write_preamble(out: &mut Vec<u8>, cost: u32, global_idx: u32, stack_limit: u32) {
	// stack_height += cost
	out.push(0x23); // global.get
	write_u32(out, global_idx);
	out.push(0x41); // i32.const
	write_i32(out, cost as i32);
	out.push(0x6a); // i32.add
	out.push(0x24); // global.set
	write_u32(out, global_idx);
	// if stack_height > limit: unreachable
	out.push(0x23); // global.get
	write_u32(out, global_idx);
	out.push(0x41); // i32.const
	write_i32(out, stack_limit as i32);
	out.push(0x4b); // i32.gt_u
	out.extend_from_slice(&[0x04, 0x40]); // if (no result)
	out.push(0x00); // unreachable
	out.push(0x0b); // end
}

fn write_postamble(out: &mut Vec<u8>, cost: u32, global_idx: u32) {
	// stack_height -= cost
	out.push(0x23); // global.get
	write_u32(out, global_idx);
	out.push(0x41); // i32.const
	write_i32(out, cost as i32);
	out.push(0x6b); // i32.sub
	out.push(0x24); // global.set
	write_u32(out, global_idx);
}

/// Wraps each call in `raw` (a function body without its size prefix) whose callee has a
/// non-zero cost. Returns `None` if there is no such call.
fn instrument_body(
	raw: &[u8],
	body: &wm::DecodedBody,
	costs: &[u32],
	global_idx: u32,
	stack_limit: u32,
) -> Option<Vec<u8>> {
	let mut out: Option<Vec<u8>> = None;
	let mut copied = 0;
	for (i, (instr, offset)) in body.instrs.iter().enumerate() {
		let Instr::Call(callee) = instr else { continue };
		let Some(&cost) = costs.get(*callee as usize) else { continue };
		if cost == 0 {
			continue;
		}
		// A call is never the last instruction: the body ends with `end`.
		let next = body.instrs.get(i + 1).map_or(raw.len(), |(_, o)| *o);
		let out = out.get_or_insert_with(|| Vec::with_capacity(raw.len() + raw.len() / 2));
		out.extend_from_slice(&raw[copied..*offset]);
		write_preamble(out, cost, global_idx, stack_limit);
		out.extend_from_slice(&raw[*offset..next]);
		write_postamble(out, cost, global_idx);
		copied = next;
	}
	out.map(|mut out| {
		out.extend_from_slice(&raw[copied..]);
		out
	})
}

fn val_type_byte(ty: &ValType) -> u8 {
	match ty {
		ValType::I32 => 0x7f,
		ValType::I64 => 0x7e,
		ValType::F32 => 0x7d,
		ValType::F64 => 0x7c,
		// `WasmModule::parse` admits MVP value types only.
		_ => unreachable!("non-MVP value type in a parsed module"),
	}
}

fn encode_signature(out: &mut Vec<u8>, sig: &Signature) {
	out.push(0x60);
	write_u32(out, sig.params.len() as u32);
	out.extend(sig.params.iter().map(val_type_byte));
	write_u32(out, sig.results.len() as u32);
	out.extend(sig.results.iter().map(val_type_byte));
}

fn item_count(payload: &[u8]) -> Result<u32> {
	wm::split_count(payload).map(|(count, _)| count)
}

/// Points exports, table entries and the start function at thunks that meter the call, and
/// reassembles the module the way `parity-wasm`'s builder did.
fn generate_thunks(
	module: WasmModule,
	ctx: &Context,
	costs: &[u32],
	global_idx: u32,
	stack_limit: u32,
) -> Result<WasmModule> {
	let mut known: [Option<Vec<u8>>; 12] = Default::default();
	// Custom sections, and anything else the builder had no slot for, in reverse order.
	let mut others = Vec::new();
	for section in module.sections.into_iter().rev() {
		match section {
			Section::Known { id, payload } if (id::TYPE..=id::DATA).contains(&id) => {
				known[id as usize] = Some(payload)
			},
			other => others.push(other),
		}
	}

	let mut exports = known[id::EXPORT as usize].as_deref().map(exports).transpose()?;
	let mut elements = known[id::ELEMENT as usize].as_deref().map(element_segments).transpose()?;
	let mut start = known[id::START as usize].as_deref().map(start_function).transpose()?;

	// Every function entered from outside, keyed (and so ordered) by function index.
	let mut replacement: BTreeMap<u32, (Signature, u32)> = BTreeMap::new();
	let exported = exports
		.iter()
		.flatten()
		.filter(|e| e.kind == ExternalKind::Func)
		.map(|e| e.index);
	let in_table = elements.iter().flatten().flat_map(|s| s.members.iter().copied());
	for func_idx in exported.chain(in_table).chain(start) {
		let cost = *costs.get(func_idx as usize).ok_or("function index isn't found")?;
		if cost != 0 {
			replacement.insert(func_idx, (ctx.resolve_func_type(func_idx)?.clone(), cost));
		}
	}

	let mut types = ctx.types.clone();
	let mut new_types = Vec::new();
	let mut new_functions = Vec::new();
	let mut new_bodies = Vec::new();
	let mut thunk_idx = BTreeMap::new();
	let mut next_func_idx = (ctx.imported.len() + ctx.defined.len()) as u32;
	for (func_idx, (signature, cost)) in &replacement {
		// The first structurally equal type, or a new one.
		let type_idx = match types.iter().position(|t| t == signature) {
			Some(idx) => idx as u32,
			None => {
				encode_signature(&mut new_types, signature);
				types.push(signature.clone());
				(types.len() - 1) as u32
			},
		};
		write_u32(&mut new_functions, type_idx);

		let mut thunk = vec![0x00]; // no locals
		for param_idx in 0..signature.params.len() as u32 {
			thunk.push(0x20); // local.get
			write_u32(&mut thunk, param_idx);
		}
		write_preamble(&mut thunk, *cost, global_idx, stack_limit);
		thunk.push(0x10); // call
		write_u32(&mut thunk, *func_idx);
		write_postamble(&mut thunk, *cost, global_idx);
		thunk.push(0x0b); // end
		write_u32(&mut new_bodies, thunk.len() as u32);
		new_bodies.extend_from_slice(&thunk);

		thunk_idx.insert(*func_idx, next_func_idx);
		next_func_idx += 1;
	}

	let added = replacement.len() as u32;
	let append = |section: &mut Option<Vec<u8>>, count: u32, items: &[u8]| -> Result<()> {
		if count > 0 {
			let payload = section.get_or_insert_with(|| vec![0]);
			*payload = wm::append_items(payload, count, items)?;
		}
		Ok(())
	};
	append(&mut known[id::TYPE as usize], (types.len() - ctx.types.len()) as u32, &new_types)?;
	append(&mut known[id::FUNCTION as usize], added, &new_functions)?;
	append(&mut known[id::CODE as usize], added, &new_bodies)?;

	// Re-point everything that enters from outside at its thunk.
	let fixup = |func_idx: &mut u32| {
		if let Some(thunk) = thunk_idx.get(func_idx) {
			*func_idx = *thunk;
		}
	};
	if let Some(exports) = exports.as_mut() {
		exports
			.iter_mut()
			.filter(|e| e.kind == ExternalKind::Func)
			.for_each(|e| fixup(&mut e.index));
		known[id::EXPORT as usize] = Some(wm::encode_vec(exports, |out, e| e.encode(out)));
	}
	if let Some(elements) = elements.as_mut() {
		elements.iter_mut().flat_map(|s| s.members.iter_mut()).for_each(fixup);
		known[id::ELEMENT as usize] = Some(wm::encode_vec(elements, |out, s| s.encode(out)));
	}
	if let Some(start) = start.as_mut() {
		fixup(start);
	}

	// Known sections in order, each only if it has entries; then the rest.
	let mut sections = Vec::new();
	for section_id in id::TYPE..=id::DATA {
		if section_id == id::START {
			if let Some(start) = start {
				let mut payload = Vec::new();
				write_u32(&mut payload, start);
				sections.push(Section::Known { id: id::START, payload });
			}
			continue;
		}
		if let Some(payload) = known[section_id as usize].take() {
			if item_count(&payload)? > 0 {
				sections.push(Section::Known { id: section_id, payload });
			}
		}
	}
	sections.extend(others);

	Ok(WasmModule { sections })
}
