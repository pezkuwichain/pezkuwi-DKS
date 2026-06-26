//! # FRAME Runtime Types
//!
//! This reference document briefly explores the idea around types generated at the runtime level by
//! the FRAME macros.
//!
//! > As of now, many of these important types are generated within the internals of
//! > [`construct_runtime`], and there is no easy way for you to visually know they exist.
//! > [#pezkuwi-sdk#1378](https://github.com/pezkuwichain/pezkuwi-sdk/issues/251) is meant to
//! > significantly improve this. Exploring the rust-docs of a runtime, such as [`runtime`] which is
//! > defined in this module is as of now the best way to learn about these types.
//!
//! ## Composite Enums
//!
//! Many types within a FRAME runtime follow the following structure:
//!
//! * Each individual pezpallet defines a type, for example `Foo`.
//! * At the runtime level, these types are amalgamated into a single type, for example
//!   `RuntimeFoo`.
//!
//! As the names suggest, all composite enums in a FRAME runtime start their name with `Runtime`.
//! For example, `RuntimeCall` is a representation of the most high level `Call`-able type in the
//! runtime.
//!
//! Composite enums are generally convertible to their individual parts as such:
#![doc = simple_mermaid::mermaid!("../../../mermaid/outer_runtime_types.mmd")]
//!
//! In that one can always convert from the inner type into the outer type, but not vice versa. This
//! is usually expressed by implementing `From`, `TryFrom`, `From<Result<_>>` and similar traits.
//!
//! ### Example
//!
//! We provide the following two pallets: [`pezpallet_foo`] and [`pezpallet_bar`]. Each define a
//! dispatchable, and `Foo` also defines a custom origin. Lastly, `Bar` defines an additional
//! `GenesisConfig`.
#![doc = docify::embed!("./src/reference_docs/frame_runtime_types.rs", pezpallet_foo)]
#![doc = docify::embed!("./src/reference_docs/frame_runtime_types.rs", pezpallet_bar)]
//!
//! Let's explore how each of these affect the [`RuntimeCall`], [`RuntimeOrigin`] and
//! [`RuntimeGenesisConfig`] generated in [`runtime`] respectively.
//!
//! As observed, [`RuntimeCall`] has 3 variants, one for each pezpallet and one for
//! `pezframe_system`. If you explore further, you will soon realize that each variant is merely a
//! pointer to the `Call` type in each pezpallet, for example [`pezpallet_foo::Call`].
//!
//! [`RuntimeOrigin`]'s [`OriginCaller`] has two variants, one for system, and one for
//! `pezpallet_foo` which utilized [`pezframe::pezpallet_macros::origin`].
//!
//! Finally, [`RuntimeGenesisConfig`] is composed of `pezframe_system` and a variant for
//! `pezpallet_bar`'s [`pezpallet_bar::GenesisConfig`].
//!
//! You can find other composite enums by scanning [`runtime`] for other types who's name starts
//! with `Runtime`. Some of the more noteworthy ones are:
//!
//! - [`RuntimeEvent`]
//! - [`RuntimeError`]
//! - [`RuntimeHoldReason`]
//!
//! ### Adding Further Constraints to Runtime Composite Enums
//!
//! This section explores a common scenario where a pezpallet has access to one of these runtime
//! composite enums, but it wishes to further specify it by adding more trait bounds to it.
//!
//! Let's take the example of `RuntimeCall`. This is an associated type in
//! [`pezframe_system::Config::RuntimeCall`], and all pallets have access to this type, because they
//! have access to [`pezframe_system::Config`]. Finally, this type is meant to be set to outer call
//! of the entire runtime.
//!
//! But, let's not forget that this is information that *we know*, and the Rust compiler does not.
//! All that the rust compiler knows about this type is *ONLY* what the trait bounds of
//! [`pezframe_system::Config::RuntimeCall`] are specifying:
#![doc = docify::embed!("../../bizinikiwi/pezframe/system/src/lib.rs", system_runtime_call)]
//!
//! So, when at a given pezpallet, one accesses `<T as pezframe_system::Config>::RuntimeCall`, the
//! type is extremely opaque from the perspective of the Rust compiler.
//!
//! How can a pezpallet access the `RuntimeCall` type with further constraints? For example, each
//! pezpallet has its own `enum Call`, and knows that its local `Call` is a part of `RuntimeCall`,
//! therefore there should be a `impl From<Call<_>> for RuntimeCall`.
//!
//! The only way to express this using Rust's associated types is for the pezpallet to **define its
//! own associated type `RuntimeCall`, and further specify what it thinks `RuntimeCall` should be**.
//!
//! In this case, we will want to assert the existence of [`pezframe::traits::IsSubType`], which is
//! very similar to [`TryFrom`].
#![doc = docify::embed!("./src/reference_docs/frame_runtime_types.rs", custom_runtime_call)]
//!
//! And indeed, at the runtime level, this associated type would be the same `RuntimeCall` that is
//! passed to `pezframe_system`.
#![doc = docify::embed!("./src/reference_docs/frame_runtime_types.rs", pezpallet_with_specific_runtime_call_impl)]
//!
//! > In other words, the degree of specificity that [`pezframe_system::Config::RuntimeCall`] has is
//! > not enough for the pezpallet to work with. Therefore, the pezpallet has to define its own
//! > associated
//! > type representing `RuntimeCall`.
//!
//! Another way to look at this is:
//!
//! `pezpallet_with_specific_runtime_call::Config::RuntimeCall` and
//! `pezframe_system::Config::RuntimeCall` are two different representations of the same concrete
//! type that is only known when the runtime is being constructed.
//!
//! Now, within this pezpallet, this new `RuntimeCall` can be used, and it can use its new trait
//! bounds, such as being [`pezframe::traits::IsSubType`]:
#![doc = docify::embed!("./src/reference_docs/frame_runtime_types.rs", custom_runtime_call_usages)]
//!
//! > Once Rust's "_Associated Type Bounds RFC_" is usable, this syntax can be used to
//! > simplify the above scenario. See [this](https://github.com/pezkuwichain/pezkuwi-sdk/issues/278)
//! > issue for more information.
//!
//! ### Asserting Equality of Multiple Runtime Composite Enums
//!
//! Recall that in the above example, `<T as Config>::RuntimeCall` and `<T as
//! pezframe_system::Config>::RuntimeCall` are expected to be equal types, but at the compile-time
//! we have to represent them with two different associated types with different bounds. Would it
//! not be cool if we had a test to make sure they actually resolve to the same concrete type once
//! the runtime is constructed? The following snippet exactly does that:
#![doc = docify::embed!("./src/reference_docs/frame_runtime_types.rs", assert_equality)]
//!
//! We leave it to the reader to further explore what [`pezframe::traits::Hooks::integrity_test`] is,
//! and what [`core::any::TypeId`] is. Another way to assert this is using
//! [`pezframe::traits::IsType`].
//!
//! ## Type Aliases
//!
//! A number of type aliases are generated by the `construct_runtime` which are also noteworthy:
//!
//! * [`runtime::PalletFoo`] is an alias to [`pezpallet_foo::Pezpallet`]. Same for `PalletBar`, and
//!   `System`
//! * [`runtime::AllPalletsWithSystem`] is an alias for a tuple of all of the above. This type is
//!   important to FRAME internals such as `executive`, as it implements traits such as
//!   [`pezframe::traits::Hooks`].
//!
//! ## Further Details
//!
//! * [`crate::reference_docs::frame_origin`] explores further details about the usage of
//!   `RuntimeOrigin`.
//! * [`RuntimeCall`] is a particularly interesting composite enum as it dictates the encoding of an
//!   extrinsic. See [`crate::reference_docs::transaction_extensions`] for more information.
//! * See the documentation of [`construct_runtime`].
//! * See the corresponding lecture in the [PBA Lectures](https://www.youtube.com/watch?v=OCBC1pMYPoc&list=PL-w_i5kwVqbni1Ch2j_RwTIXiB-bwnYqq&index=11).
//!
//!
//! [`construct_runtime`]: pezframe::runtime::prelude::construct_runtime
//! [`runtime::PalletFoo`]: crate::reference_docs::frame_runtime_types::runtime::PalletFoo
//! [`runtime::AllPalletsWithSystem`]: crate::reference_docs::frame_runtime_types::runtime::AllPalletsWithSystem
//! [`runtime`]: crate::reference_docs::frame_runtime_types::runtime
//! [`pezpallet_foo`]: crate::reference_docs::frame_runtime_types::pezpallet_foo
//! [`pezpallet_foo::Call`]: crate::reference_docs::frame_runtime_types::pezpallet_foo::Call
//! [`pezpallet_foo::Pezpallet`]: crate::reference_docs::frame_runtime_types::pezpallet_foo::Pezpallet
//! [`pezpallet_bar`]: crate::reference_docs::frame_runtime_types::pezpallet_bar
//! [`pezpallet_bar::GenesisConfig`]: crate::reference_docs::frame_runtime_types::pezpallet_bar::GenesisConfig
//! [`RuntimeEvent`]: crate::reference_docs::frame_runtime_types::runtime::RuntimeEvent
//! [`RuntimeGenesisConfig`]:
//!     crate::reference_docs::frame_runtime_types::runtime::RuntimeGenesisConfig
//! [`RuntimeOrigin`]: crate::reference_docs::frame_runtime_types::runtime::RuntimeOrigin
//! [`OriginCaller`]: crate::reference_docs::frame_runtime_types::runtime::OriginCaller
//! [`RuntimeError`]: crate::reference_docs::frame_runtime_types::runtime::RuntimeError
//! [`RuntimeCall`]: crate::reference_docs::frame_runtime_types::runtime::RuntimeCall
//! [`RuntimeHoldReason`]: crate::reference_docs::frame_runtime_types::runtime::RuntimeHoldReason

use pezframe::prelude::*;

#[docify::export]
#[pezframe::pezpallet(dev_mode)]
pub mod pezpallet_foo {
	use super::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::origin]
	#[derive(
		PartialEq,
		Eq,
		Clone,
		RuntimeDebug,
		Encode,
		Decode,
		DecodeWithMemTracking,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub enum Origin {
		A,
		B,
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		pub fn foo(_origin: OriginFor<T>) -> DispatchResult {
			todo!();
		}

		pub fn other(_origin: OriginFor<T>) -> DispatchResult {
			todo!();
		}
	}
}

#[docify::export]
#[pezframe::pezpallet(dev_mode)]
pub mod pezpallet_bar {
	use super::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::genesis_config]
	#[derive(DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub initial_account: Option<T::AccountId>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {}
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		pub fn bar(_origin: OriginFor<T>) -> DispatchResult {
			todo!();
		}
	}
}

pub mod runtime {
	use super::{pezpallet_bar, pezpallet_foo};
	use pezframe::{runtime::prelude::*, testing_prelude::*};

	#[docify::export(runtime_exp)]
	construct_runtime!(
		pub struct Runtime {
			System: pezframe_system,
			PalletFoo: pezpallet_foo,
			PalletBar: pezpallet_bar,
		}
	);

	#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
	impl pezframe_system::Config for Runtime {
		type Block = MockBlock<Self>;
	}

	impl pezpallet_foo::Config for Runtime {}
	impl pezpallet_bar::Config for Runtime {}
}

#[pezframe::pezpallet(dev_mode)]
pub mod pezpallet_with_specific_runtime_call {
	use super::*;
	use pezframe::traits::IsSubType;

	#[docify::export(custom_runtime_call)]
	/// A pezpallet that wants to further narrow down what `RuntimeCall` is.
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		type RuntimeCall: IsSubType<Call<Self>>;
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	// note that this pezpallet needs some `call` to have a `enum Call`.
	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		pub fn foo(_origin: OriginFor<T>) -> DispatchResult {
			todo!();
		}
	}

	#[docify::export(custom_runtime_call_usages)]
	impl<T: Config> Pezpallet<T> {
		fn _do_something_useful_with_runtime_call(call: <T as Config>::RuntimeCall) {
			// check if the runtime call given is of this pezpallet's variant.
			let _maybe_my_call: Option<&Call<T>> = call.is_sub_type();
			todo!();
		}
	}

	#[docify::export(assert_equality)]
	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		fn integrity_test() {
			use core::any::TypeId;
			assert_eq!(
				TypeId::of::<<T as Config>::RuntimeCall>(),
				TypeId::of::<<T as pezframe_system::Config>::RuntimeCall>()
			);
		}
	}
}

pub mod runtime_with_specific_runtime_call {
	use super::pezpallet_with_specific_runtime_call;
	use pezframe::{runtime::prelude::*, testing_prelude::*};

	construct_runtime!(
		pub struct Runtime {
			System: pezframe_system,
			PalletWithSpecificRuntimeCall: pezpallet_with_specific_runtime_call,
		}
	);

	#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
	impl pezframe_system::Config for Runtime {
		type Block = MockBlock<Self>;
	}

	#[docify::export(pezpallet_with_specific_runtime_call_impl)]
	impl pezpallet_with_specific_runtime_call::Config for Runtime {
		// an implementation of `IsSubType` is provided by `construct_runtime`.
		type RuntimeCall = RuntimeCall;
	}
}
