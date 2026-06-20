//! # FRAME Pezpallet Coupling
//!
//! This reference document explains how FRAME pallets can be combined to interact together.
//!
//! It is suggested to re-read [`crate::pezkuwi_sdk::frame_runtime`], notably the information
//! around [`pezframe::pezpallet_macros::config`]. Recall that:
//!
//! > Configuration trait of a pezpallet: It allows a pezpallet to receive types at a later
//! > point from the runtime that wishes to contain it. It allows the pezpallet to be parameterized
//! > over both types and values.
//!
//! ## Context, Background
//!
//! FRAME pallets, as per described in [`crate::pezkuwi_sdk::frame_runtime`] are:
//!
//! > A pezpallet is a unit of encapsulated logic. It has a clearly defined responsibility and can
//! > be
//! linked to other pallets.
//!
//! That is to say:
//!
//! * *encapsulated*: Ideally, a FRAME pezpallet contains encapsulated logic which has clear
//!   boundaries. It is generally a bad idea to build a single monolithic pezpallet that does
//!   multiple things, such as handling currencies, identities and staking all at the same time.
//! * *linked to other pallets*: But, adhering extensively to the above also hinders the ability to
//!   write useful applications. Pallets often need to work with each other, communicate and use
//!   each other's functionalities.
//!
//! The broad principle that allows pallets to be linked together is the same way through which a
//! pezpallet uses its `Config` trait to receive types and values from the runtime that contains it.
//!
//! There are generally two ways to achieve this:
//!
//! 1. Tight coupling pallets.
//! 2. Loose coupling pallets.
//!
//! To explain the difference between the two, consider two pallets, `A` and `B`. In both cases, `A`
//! wants to use some functionality exposed by `B`.
//!
//! When tightly coupling pallets, `A` can only exist in a runtime if `B` is also present in the
//! same runtime. That is, `A` is expressing that can only work if `B` is present.
//!
//! This translates to the following Rust code:
//!
//! ```
//! trait Pallet_B_Config {}
//! trait Pallet_A_Config: Pallet_B_Config {}
//! ```
//!
//! Contrary, when pallets are loosely coupled, `A` expresses that some functionality, expressed via
//! a trait `F`, needs to be fulfilled. This trait is then implemented by `B`, and the two pallets
//! are linked together at the runtime level. This means that `A` only relies on the implementation
//! of `F`, which may be `B`, or another implementation of `F`.
//!
//! This translates to the following Rust code:
//!
//! ```
//! trait F {}
//! trait Pallet_A_Config {
//!    type F: F;
//! }
//! // Pallet_B will implement and fulfill `F`.
//! ```
//!
//! ## Example
//!
//! Consider the following example, in which `pezpallet-foo` needs another pezpallet to provide the
//! block author to it, and `pezpallet-author` which has access to this information.
#![doc = docify::embed!("./src/reference_docs/frame_pallet_coupling.rs", pezpallet_foo)]
#![doc = docify::embed!("./src/reference_docs/frame_pallet_coupling.rs", pezpallet_author)]
//!
//! ### Tight Coupling Pallets
//!
//! To tightly couple `pezpallet-foo` and `pezpallet-author`, we use Rust's supertrait system. When
//! a pezpallet makes its own `trait Config` be bounded by another pezpallet's `trait Config`, it is
//! expressing two things:
//!
//! 1. That it can only exist in a runtime if the other pezpallet is also present.
//! 2. That it can use the other pezpallet's functionality.
//!
//! `pezpallet-foo`'s `Config` would then look like:
#![doc = docify::embed!("./src/reference_docs/frame_pallet_coupling.rs", tight_config)]
//!
//! And `pezpallet-foo` can use the method exposed by `pezpallet_author::Pezpallet` directly:
#![doc = docify::embed!("./src/reference_docs/frame_pallet_coupling.rs", tight_usage)]
//!
//!
//! ### Loosely  Coupling Pallets
//!
//! If `pezpallet-foo` wants to *not* rely on `pezpallet-author` directly, it can leverage its
//! `Config`'s associated types. First, we need a trait to express the functionality that
//! `pezpallet-foo` wants to obtain:
#![doc = docify::embed!("./src/reference_docs/frame_pallet_coupling.rs", AuthorProvider)]
//!
//! > We sometimes refer to such traits that help two pallets interact as "glue traits".
//!
//! Next, `pezpallet-foo` states that it needs this trait to be provided to it, at the runtime
//! level, via an associated type:
#![doc = docify::embed!("./src/reference_docs/frame_pallet_coupling.rs", loose_config)]
//!
//! Then, `pezpallet-foo` can use this trait to obtain the block author, without knowing where it
//! comes from:
#![doc = docify::embed!("./src/reference_docs/frame_pallet_coupling.rs", loose_usage)]
//!
//! Then, if `pezpallet-author` implements this glue-trait:
#![doc = docify::embed!("./src/reference_docs/frame_pallet_coupling.rs", pezpallet_author_provider)]
//!
//! And upon the creation of the runtime, the two pallets are linked together as such:
#![doc = docify::embed!("./src/reference_docs/frame_pallet_coupling.rs", runtime_author_provider)]
//!
//! Crucially, when using loose coupling, we gain the flexibility of providing different
//! implementations of `AuthorProvider`, such that different users of a `pezpallet-foo` can use
//! different ones, without any code change being needed. For example, in the code snippets of this
//! module, you can find [`OtherAuthorProvider`], which is an alternative implementation of
//! [`AuthorProvider`].
#![doc = docify::embed!("./src/reference_docs/frame_pallet_coupling.rs", other_author_provider)]
//!
//! A common pattern in pezkuwi-sdk is to provide an implementation of such glu traits for the unit
//! type as a "default/test behavior".
#![doc = docify::embed!("./src/reference_docs/frame_pallet_coupling.rs", unit_author_provider)]
//!
//! ## Frame System
//!
//! With the above information in context, we can conclude that **`pezframe_system` is a special
//! pezpallet that is tightly coupled with every other pezpallet**. This is because it provides the
//! fundamental system functionality that every pezpallet needs, such as some types like
//! [`pezframe::prelude::pezframe_system::Config::AccountId`],
//! [`pezframe::prelude::pezframe_system::Config::Hash`], and some functionality such as block number,
//! etc.
//!
//! ## Recap
//!
//! To recap, consider the following rules of thumb:
//!
//! * In all cases, try and break down big pallets apart with clear boundaries of responsibility. In
//!   general, it is easier to argue about multiple pezpallet if they only communicate together via
//!   a known trait, rather than having access to all of each others public items, such as storage
//!   and dispatchables.
//! * If a group of pallets is meant to work together, but is not foreseen to be generalized, or
//!   used by others, consider tightly coupling pallets, *if it simplifies the development*.
//! * If a pezpallet needs a functionality provided by another pezpallet, but multiple
//!   implementations can be foreseen, consider loosely coupling pallets.
//!
//! For example, all pallets in `pezkuwi-sdk` that needed to work with currencies could have been
//! tightly coupled with [`pezpallet_balances`]. But, `pezkuwi-sdk` also provides
//! [`pezpallet_assets`] (and more implementations by the community), therefore all pallets use
//! traits to loosely couple with balances or assets pezpallet. More on this in
//! [`crate::reference_docs::frame_tokens`].
//!
//! ## Further References
//!
//! - <https://www.youtube.com/watch?v=0eNGZpNkJk4>
//! - <https://exchange.pezkuwichain.app/questions/922/pezpallet-loose-couplingtight-coupling-and-missing-traits>
//!
//! [`AuthorProvider`]: crate::reference_docs::frame_pallet_coupling::AuthorProvider
//! [`OtherAuthorProvider`]: crate::reference_docs::frame_pallet_coupling::OtherAuthorProvider

#![allow(unused)]

use pezframe::prelude::*;

#[docify::export]
#[pezframe::pezpallet]
pub mod pezpallet_foo {
	use super::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	impl<T: Config> Pezpallet<T> {
		fn do_stuff_with_author() {
			// needs block author here
		}
	}
}

#[docify::export]
#[pezframe::pezpallet]
pub mod pezpallet_author {
	use super::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	impl<T: Config> Pezpallet<T> {
		pub fn author() -> T::AccountId {
			todo!("somehow has access to the block author and can return it here")
		}
	}
}

#[pezframe::pezpallet]
pub mod pezpallet_foo_tight {
	use super::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[docify::export(tight_config)]
	/// This pezpallet can only live in a runtime that has both `pezframe_system` and
	/// `pezpallet_author`.
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config + pezpallet_author::Config {}

	#[docify::export(tight_usage)]
	impl<T: Config> Pezpallet<T> {
		// anywhere in `pezpallet-foo`, we can call into `pezpallet-author` directly, namely because
		// `T: pezpallet_author::Config`
		fn do_stuff_with_author() {
			let _ = pezpallet_author::Pezpallet::<T>::author();
		}
	}
}

#[docify::export]
/// Abstraction over "something that can provide the block author".
pub trait AuthorProvider<AccountId> {
	fn author() -> AccountId;
}

#[pezframe::pezpallet]
pub mod pezpallet_foo_loose {
	use super::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[docify::export(loose_config)]
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		/// This pezpallet relies on the existence of something that implements [`AuthorProvider`],
		/// which may or may not be `pezpallet-author`.
		type AuthorProvider: AuthorProvider<Self::AccountId>;
	}

	#[docify::export(loose_usage)]
	impl<T: Config> Pezpallet<T> {
		fn do_stuff_with_author() {
			let _ = T::AuthorProvider::author();
		}
	}
}

#[docify::export(pezpallet_author_provider)]
impl<T: pezpallet_author::Config> AuthorProvider<T::AccountId> for pezpallet_author::Pezpallet<T> {
	fn author() -> T::AccountId {
		pezpallet_author::Pezpallet::<T>::author()
	}
}

pub struct OtherAuthorProvider;

#[docify::export(other_author_provider)]
impl<AccountId> AuthorProvider<AccountId> for OtherAuthorProvider {
	fn author() -> AccountId {
		todo!("somehow get the block author here")
	}
}

#[docify::export(unit_author_provider)]
impl<AccountId> AuthorProvider<AccountId> for () {
	fn author() -> AccountId {
		todo!("somehow get the block author here")
	}
}

pub mod runtime {
	use super::*;
	use pezcumulus_pezpallet_aura_ext::pezpallet;
	use pezframe::{runtime::prelude::*, testing_prelude::*};

	construct_runtime!(
		pub struct Runtime {
			System: pezframe_system,
			PalletFoo: pezpallet_foo_loose,
			PalletAuthor: pezpallet_author,
		}
	);

	#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
	impl pezframe_system::Config for Runtime {
		type Block = MockBlock<Self>;
	}

	impl pezpallet_author::Config for Runtime {}

	#[docify::export(runtime_author_provider)]
	impl pezpallet_foo_loose::Config for Runtime {
		type AuthorProvider = pezpallet_author::Pezpallet<Runtime>;
		// which is also equivalent to
		// type AuthorProvider = PalletAuthor;
	}
}
