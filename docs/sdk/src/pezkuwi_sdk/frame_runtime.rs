//! # FRAME
//!
//! ```no_compile
//!   ______   ______    ________   ___ __ __   ______
//!  /_____/\ /_____/\  /_______/\ /__//_//_/\ /_____/\
//!  \::::_\/_\:::_ \ \ \::: _  \ \\::\| \| \ \\::::_\/_
//!   \:\/___/\\:(_) ) )_\::(_)  \ \\:.      \ \\:\/___/\
//!    \:::._\/ \: __ `\ \\:: __  \ \\:.\-/\  \ \\::___\/_
//!     \:\ \    \ \ `\ \ \\:.\ \  \ \\. \  \  \ \\:\____/\
//!      \_\/     \_\/ \_\/ \__\/\__\/ \__\/ \__\/ \_____\/
//! ```
//!
//! > **F**ramework for **R**untime **A**ggregation of **M**odularized **E**ntities: Bizinikiwi's
//! > State Transition Function (Runtime) Framework.
//!
//! ## Introduction
//!
//! As described in [`crate::reference_docs::wasm_meta_protocol`], at a high-level Bizinikiwi-based
//! blockchains are composed of two parts:
//!
//! 1. A *runtime* which represents the state transition function (i.e. "Business Logic") of a
//! blockchain, and is encoded as a WASM blob.
//! 2. A node whose primary purpose is to execute the given runtime.
#![doc = simple_mermaid::mermaid!("../../../mermaid/bizinikiwi_simple.mmd")]
//!
//! *FRAME is the Bizinikiwi's framework of choice to build a runtime.*
//!
//! FRAME is composed of two major components, **pallets** and a **runtime**.
//!
//! ## Pallets
//!
//! A pezpallet is a unit of encapsulated logic. It has a clearly defined responsibility and can be
//! linked to other pallets. In order to be reusable, pallets shipped with FRAME strive to only care
//! about its own responsibilities and make as few assumptions about the general runtime as
//! possible. A pezpallet is analogous to a _module_ in the runtime.
//!
//! A pezpallet is defined as a `mod pezpallet` wrapped by the [`pezframe::pezpallet`] macro. Within
//! this macro, pezpallet components/parts can be defined. Most notable of these parts are:
//!
//! - [Config](pezframe::pezpallet_macros::config), allowing a pezpallet to make itself configurable
//!   and generic over types, values and such.
//! - [Storage](pezframe::pezpallet_macros::storage), allowing a pezpallet to define onchain storage.
//! - [Dispatchable function](pezframe::pezpallet_macros::call), allowing a pezpallet to define
//!   extrinsics that are callable by end users, from the outer world.
//! - [Events](pezframe::pezpallet_macros::event), allowing a pezpallet to emit events.
//! - [Errors](pezframe::pezpallet_macros::error), allowing a pezpallet to emit well-formed errors.
//!
//! Some of these pezpallet components resemble the building blocks of a smart contract. While both
//! models are programming state transition functions of blockchains, there are crucial differences
//! between the two. See [`crate::reference_docs::runtime_vs_smart_contract`] for more.
//!
//! Most of these components are defined using macros, the full list of which can be found in
//! [`pezframe::pezpallet_macros`].
//!
//! ### Example
//!
//! The following example showcases a minimal pezpallet.
#![doc = docify::embed!("src/pezkuwi_sdk/frame_runtime.rs", pezpallet)]
//!
//! ## Runtime
//!
//! A runtime is a collection of pallets that are amalgamated together. Each pezpallet typically has
//! some configurations (exposed as a `trait Config`) that needs to be *specified* in the runtime.
//! This is done with [`pezframe::runtime::prelude::construct_runtime`].
//!
//! A (real) runtime that actually wishes to compile to WASM needs to also implement a set of
//! runtime-apis. These implementation can be specified using the
//! [`pezframe::runtime::prelude::impl_runtime_apis`] macro.
//!
//! ### Example
//!
//! The following example shows a (test) runtime that is composing the pezpallet demonstrated above,
//! next to the [`pezframe::prelude::pezframe_system`] pezpallet, into a runtime.
#![doc = docify::embed!("src/pezkuwi_sdk/frame_runtime.rs", runtime)]
//!
//! ## More Examples
//!
//! You can find more FRAME examples that revolve around specific features at
//! [`pezpallet_examples`].
//!
//! ## Alternatives 🌈
//!
//! There is nothing in the Bizinikiwi's node side code-base that mandates the use of FRAME. While
//! FRAME makes it very simple to write Bizinikiwi-based runtimes, it is by no means intended to be
//! the only one. At the end of the day, any WASM blob that exposes the right set of runtime APIs is
//! a valid Runtime form the point of view of a Bizinikiwi client (see
//! [`crate::reference_docs::wasm_meta_protocol`]). Notable examples are:
//!
//! * writing a runtime in pure Rust, as done in [this template](https://github.com/JoshOrndorff/frameless-node-template).
//! * writing a runtime in AssemblyScript, as explored in [this project](https://github.com/LimeChain/subsembly).

/// A FRAME based pezpallet. This `mod` is the entry point for everything else. All
/// `#[pezpallet::xxx]` macros must be defined in this `mod`. Although, frame also provides an
/// experimental feature to break these parts into different `mod`s. See [`pezpallet_examples`] for
/// more.
#[docify::export]
#[pezframe::pezpallet(dev_mode)]
pub mod pezpallet {
	use pezframe::prelude::*;

	/// The configuration trait of a pezpallet. Mandatory. Allows a pezpallet to receive types at a
	/// later point from the runtime that wishes to contain it. It allows the pezpallet to be
	/// parameterized over both types and values.
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		/// A type that is not known now, but the runtime that will contain this pezpallet will
		/// know it later, therefore we define it here as an associated type.
		#[allow(deprecated)]
		type RuntimeEvent: IsType<<Self as pezframe_system::Config>::RuntimeEvent>
			+ From<Event<Self>>;

		/// A parameterize-able value that we receive later via the `Get<_>` trait.
		type ValueParameter: Get<u32>;

		/// Similar to [`Config::ValueParameter`], but using `const`. Both are functionally
		/// equal, but offer different tradeoffs.
		const ANOTHER_VALUE_PARAMETER: u32;
	}

	/// A mandatory struct in each pezpallet. All functions callable by external users (aka.
	/// transactions) must be attached to this type (see [`pezframe::pezpallet_macros::call`]). For
	/// convenience, internal (private) functions can also be attached to this type.
	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(PhantomData<T>);

	/// The events that this pezpallet can emit.
	#[pezpallet::event]
	pub enum Event<T: Config> {}

	/// A storage item that this pezpallet contains. This will be part of the state root trie
	/// of the blockchain.
	#[pezpallet::storage]
	pub type Value<T> = StorageValue<Value = u32>;

	/// All *dispatchable* call functions (aka. transactions) are attached to `Pezpallet` in a
	/// `impl` block.
	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// This will be callable by external users, and has two u32s as a parameter.
		pub fn some_dispatchable(
			_origin: OriginFor<T>,
			_param: u32,
			_other_para: u32,
		) -> DispatchResult {
			Ok(())
		}
	}
}

/// A simple runtime that contains the above pezpallet and `pezframe_system`, the mandatory
/// pezpallet of all runtimes. This runtime is for testing, but it shares a lot of similarities with
/// a *real* runtime.
#[docify::export]
pub mod runtime {
	use super::pezpallet as pezpallet_example;
	use pezframe::{prelude::*, testing_prelude::*};

	// The major macro that amalgamates pallets into `enum Runtime`
	construct_runtime!(
		pub enum Runtime {
			System: pezframe_system,
			Example: pezpallet_example,
		}
	);

	// These `impl` blocks specify the parameters of each pezpallet's `trait Config`.
	#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
	impl pezframe_system::Config for Runtime {
		type Block = MockBlock<Self>;
	}

	impl pezpallet_example::Config for Runtime {
		type RuntimeEvent = RuntimeEvent;
		type ValueParameter = ConstU32<42>;
		const ANOTHER_VALUE_PARAMETER: u32 = 42;
	}
}
