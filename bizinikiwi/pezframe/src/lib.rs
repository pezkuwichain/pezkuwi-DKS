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
//! ## Usage
//!
//! This crate is organized into 3 stages:
//!
//! 1. preludes: `prelude`, `testing_prelude`, `runtime::prelude`, `benchmarking::prelude`, and
//!    `weights_prelude`.
//! 2. domain-specific modules, like `traits`, `hashing`, `arithmetic` and `derive`.
//! 3. Accessing frame/bizinikiwi dependencies directly: `deps`.
//!
//! The main intended use of this crate is through preludes, which re-export most of the components
//! needed for pezpallet development. Domain-specific modules serve as a backup for organization,
//! and `deps` provides direct access to all dependencies if needed.
//!
//!
//! ### Example Usage
//!
//! <!-- Note: This example is marked `ignore` because doc tests run in isolation and cannot
//!      properly resolve the pezpallet macro attributes without a full pallet context.
//!      The functionality is tested in the pezframe-support integration tests. -->
//! ```ignore
//! use pezframe as frame;
//!
//! #[pezframe::pezpallet]
//! pub mod pezpallet {
//! 	use pezframe::prelude::*;
//! 	// ^^ using the prelude!
//!
//! 	#[pezpallet::config]
//! 	pub trait Config: pezframe_system::Config {}
//!
//! 	#[pezpallet::pezpallet]
//! 	pub struct Pezpallet<T>(_);
//! }
//!
//! #[cfg(test)]
//! pub mod tests {
//! 	use pezframe::testing_prelude::*;
//! }
//!
//! #[cfg(feature = "runtime-benchmarks")]
//! pub mod benchmarking {
//! 	use pezframe::benchmarking::prelude::*;
//! }
//!
//! pub mod runtime {
//! 	use pezframe::runtime::prelude::*;
//! }
//! ```
//!
//! ### Features
//!
//! This crate uses a `runtime` feature to include all types and tools needed to build FRAME-based
//! runtimes. For runtime development, import it as:
//!
//! ```text
//! pezkuwi-sdk-frame = { version = "foo", features = ["runtime"] }
//! ```
//!
//! If you just want to build a pezpallet instead, import it as
//!
//! ```text
//! pezkuwi-sdk-frame = { version = "foo" }
//! ```
//!
//! ### Prelude Relationships
//!
//! The preludes have overlapping imports for convenience:
//! - `testing_prelude` includes `prelude` and `runtime::prelude`
//! - `runtime::prelude` includes `prelude`
//! - `benchmarking::prelude` includes `prelude`
//!
//! ## Naming
//!
//! Please note that this crate can only be imported as `pezkuwi-sdk-frame` or `frame`. This is due
//! to compatibility matters with `pezframe-support`.
//!
//! A typical pezpallet's `Cargo.toml` using this crate looks like:
//!
//! ```ignore
//! [dependencies]
//! codec = { features = ["max-encoded-len"], workspace = true }
//! scale-info = { features = ["derive"], workspace = true }
//! frame = { workspace = true, features = ["runtime"] }
//!
//! [features]
//! default = ["std"]
//! std = [
//! 	"codec/std",
//! 	"scale-info/std",
//! 	"frame/std",
//! ]
//! runtime-benchmarks = [
//! 	"frame/runtime-benchmarks",
//! ]
//! try-runtime = [
//! 	"frame/try-runtime",
//! ]
//! ```
//!
//! ## Documentation
//!
//! For more detailed documentation and examples, see [`pezkuwi_sdk_frame`](https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_frame/index.html).

#![cfg_attr(not(feature = "std"), no_std)]

#[doc(no_inline)]
pub use pezframe_support::pezpallet;

#[doc(no_inline)]
pub use pezframe_support::pezpallet_macros::{import_section, pezpallet_section};

/// The logging library of the runtime. Can normally be the classic `log` crate.
pub use log;

#[doc(inline)]
pub use pezframe_support::storage_alias;

/// Macros used within the main [`pezpallet`] macro.
///
/// Note: All of these macros are "stubs" and not really usable outside `#[pezpallet] mod pezpallet
/// { .. }`. They are mainly provided for documentation and IDE support.
///
/// To view a list of all the macros and their documentation, follow the links in the 'Re-exports'
/// section below:
pub mod pezpallet_macros {
	#[doc(no_inline)]
	pub use pezframe_support::{derive_impl, pezpallet, pezpallet_macros::*};
}

/// The main prelude of FRAME.
///
/// This prelude should almost always be the first line of code in any pezpallet or runtime.
///
/// ```
/// use pezframe::prelude::*;
///
/// // rest of your pezpallet..
/// mod pezpallet {}
/// ```
pub mod prelude {
	/// `pezframe_system`'s parent crate, which is mandatory in all pallets build with this
	/// crate.
	///
	/// Conveniently, the keyword `pezframe_system` is in scope as one uses `use
	/// pezframe::prelude::*`.
	#[doc(inline)]
	pub use pezframe_system;

	/// Pezpallet prelude of `pezframe-support`.
	///
	/// Note: this needs to revised once `pezframe-support` evolves.
	#[doc(no_inline)]
	pub use pezframe_support::pezpallet_prelude::*;

	/// Dispatch types from `pezframe-support`, other fundamental traits.
	#[doc(no_inline)]
	pub use pezframe_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
	pub use pezframe_support::{
		defensive, defensive_assert,
		traits::{
			Contains, Defensive, DefensiveSaturating, EitherOf, EstimateNextSessionRotation,
			Everything, InsideBoth, InstanceFilter, IsSubType, MapSuccess, NoOpPoll,
			OnRuntimeUpgrade, OneSessionHandler, PalletInfoAccess, RankedMembers,
			RankedMembersSwapHandler, VariantCount, VariantCountOf,
		},
		PalletId,
	};

	/// Pezpallet prelude of `pezframe-system`.
	#[doc(no_inline)]
	pub use pezframe_system::pezpallet_prelude::*;

	/// Transaction related helpers to submit transactions.
	#[doc(no_inline)]
	pub use pezframe_system::offchain::*;

	/// All FRAME-relevant derive macros.
	#[doc(no_inline)]
	pub use super::derive::*;

	/// All hashing related components.
	pub use super::hashing::*;

	/// All transaction related components.
	pub use crate::transaction::*;

	/// All account related components.
	pub use super::account::*;

	/// All arithmetic types and traits used for safe math.
	pub use super::arithmetic::*;

	/// All token related types and traits.
	pub use super::token::*;

	/// Runtime traits
	#[doc(no_inline)]
	pub use pezsp_runtime::traits::{
		AccountIdConversion, BlockNumberProvider, Bounded, Convert, ConvertBack, DispatchInfoOf,
		Dispatchable, ReduceBy, ReplaceWithDefault, SaturatedConversion, Saturating, StaticLookup,
		TrailingZeroInput,
	};

	/// Bounded storage related types.
	pub use pezsp_runtime::{BoundedSlice, BoundedVec};

	/// Other error/result types for runtime
	#[doc(no_inline)]
	pub use pezsp_runtime::{
		BoundToRuntimeAppPublic, DispatchErrorWithPostInfo, DispatchResultWithInfo, TokenError,
	};
}

#[cfg(any(feature = "try-runtime", test))]
pub mod try_runtime {
	pub use pezsp_runtime::TryRuntimeError;
}

/// Prelude to be included in the `benchmarking.rs` of a pezpallet.
///
/// It supports both the `benchmarking::v1::benchmarks` and `benchmarking::v2::benchmark` syntax.
///
/// ```
/// use pezframe::benchmarking::prelude::*;
/// // rest of your code.
/// ```
///
/// It already includes `pezframe::prelude::*` and `pezframe::testing_prelude`.
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking {
	mod shared {
		pub use pezframe_benchmarking::{
			add_benchmark, v1::account, whitelist, whitelisted_caller,
		};
		// all benchmarking functions.
		pub use pezframe_benchmarking::benchmarking::*;
		// The system origin, which is very often needed in benchmarking code. Might be tricky only
		// if the pezpallet defines its own `#[pezpallet::origin]` and call it `RawOrigin`.
		pub use pezframe_system::{Pezpallet as System, RawOrigin};
	}

	#[deprecated(
		note = "'The V1 benchmarking syntax is deprecated. Please use the V2 syntax. This warning may become a hard error any time after April 2025. For more info, see: https://github.com/pezkuwichain/pezkuwi-sdk/issues/268"
	)]
	pub mod v1 {
		pub use super::shared::*;
		pub use pezframe_benchmarking::benchmarks;
	}

	pub mod prelude {
		pub use crate::prelude::*;
		pub use pezframe_benchmarking::{
			add_benchmark, benchmarking::add_to_whitelist, v1::account, v2::*, whitelist,
			whitelisted_caller,
		};
		pub use pezframe_support::traits::UnfilteredDispatchable;
		pub use pezframe_system::{Pezpallet as System, RawOrigin};
	}
}

/// Prelude to be included in the `weight.rs` of each pezpallet.
///
/// ```
/// pub use pezframe::weights_prelude::*;
/// ```
pub mod weights_prelude {
	pub use core::marker::PhantomData;
	pub use pezframe_support::{
		traits::Get,
		weights::{
			constants::{ParityDbWeight, RocksDbWeight},
			Weight,
		},
	};
	pub use pezframe_system;
}

/// The main testing prelude of FRAME.
///
/// A test setup typically starts with:
///
/// ```
/// use pezframe::testing_prelude::*;
/// // rest of your test setup.
/// ```
///
/// This automatically brings in `pezframe::prelude::*` and
/// `pezframe::runtime::prelude::*`.
#[cfg(feature = "std")]
pub mod testing_prelude {
	pub use crate::{prelude::*, runtime::prelude::*};

	/// Testing includes building a runtime, so we bring in all preludes related to runtimes as
	/// well.
	pub use super::runtime::testing_prelude::*;

	/// Other helper macros from `pezframe_support` that help with asserting in tests.
	pub use pezframe_support::{
		assert_err, assert_err_ignore_postinfo, assert_error_encoded_size, assert_noop, assert_ok,
		assert_storage_noop, defensive, ensure, hypothetically, hypothetically_ok, storage_alias,
		StorageNoopGuard,
	};

	pub use pezframe_support::traits::Everything;
	pub use pezframe_system::{self, mocking::*, RunToBlockHooks};

	#[deprecated(note = "Use `pezframe::testing_prelude::TestState` instead.")]
	pub use pezsp_io::TestExternalities;

	pub use pezsp_io::TestExternalities as TestState;

	/// Commonly used runtime traits for testing.
	pub use pezsp_runtime::{traits::BadOrigin, StateVersion};
}

/// All of the types and tools needed to build FRAME-based runtimes.
#[cfg(any(feature = "runtime", feature = "std"))]
pub mod runtime {
	/// The main prelude of `FRAME` for building runtimes.
	///
	/// A runtime typically starts with:
	///
	/// ```
	/// use pezframe::runtime::prelude::*;
	/// ```
	///
	/// This automatically brings in `pezframe::prelude::*`.
	pub mod prelude {
		pub use crate::prelude::*;

		/// All of the types related to the FRAME runtime executive.
		pub use pezframe_executive::*;

		/// Macro to amalgamate the runtime into `struct Runtime`.
		///
		/// Consider using the new version of this [`frame_construct_runtime`].
		pub use pezframe_support::construct_runtime;

		/// Macro to amalgamate the runtime into `struct Runtime`.
		///
		/// This is the newer version of [`construct_runtime`].
		pub use pezframe_support::runtime as frame_construct_runtime;

		/// Macro to easily derive the `Config` trait of various pezpallet for `Runtime`.
		pub use pezframe_support::derive_impl;

		/// Macros to easily impl traits such as `Get` for types.
		// TODO: using linking in the Get in the line above triggers an ICE :/
		pub use pezframe_support::{ord_parameter_types, parameter_types};

		/// For building genesis config.
		pub use pezframe_support::genesis_builder_helper::{build_state, get_preset};

		/// Const types that can easily be used in conjuncture with `Get`.
		pub use pezframe_support::traits::{
			ConstBool, ConstI128, ConstI16, ConstI32, ConstI64, ConstI8, ConstU128, ConstU16,
			ConstU32, ConstU64, ConstU8,
		};

		/// Used for simple fee calculation.
		pub use pezframe_support::weights::{self, FixedFee, NoFee};

		/// Primary types used to parameterize `EnsureOrigin` and `EnsureRootWithArg`.
		pub use pezframe_system::{
			EnsureNever, EnsureNone, EnsureRoot, EnsureRootWithSuccess, EnsureSigned,
			EnsureSignedBy,
		};

		/// Types to define your runtime version.
		// TODO: Remove deprecation suppression once
		#[allow(deprecated)]
		pub use pezsp_version::create_runtime_str;
		pub use pezsp_version::{runtime_version, RuntimeVersion};

		#[cfg(feature = "std")]
		pub use pezsp_version::NativeVersion;

		/// Macro to implement runtime APIs.
		pub use pezsp_api::impl_runtime_apis;

		// Types often used in the runtime APIs.
		pub use pezsp_core::OpaqueMetadata;
		pub use pezsp_genesis_builder::{
			PresetId, Result as GenesisBuilderResult, DEV_RUNTIME_PRESET,
			LOCAL_TESTNET_RUNTIME_PRESET,
		};
		pub use pezsp_inherents::{CheckInherentsResult, InherentData};
		pub use pezsp_keyring::Sr25519Keyring;
		pub use pezsp_runtime::{ApplyExtrinsicResult, ExtrinsicInclusionMode};
	}

	/// Types and traits for runtimes that implement runtime APIs.
	///
	/// A testing runtime should not need this.
	///
	/// A non-testing runtime should have this enabled, as such:
	///
	/// ```
	/// use pezframe::runtime::{prelude::*, apis::{*,}};
	/// ```
	// TODO: This is because of wildcard imports, and it should be not needed once we can avoid
	// that. Imports like that are needed because we seem to need some unknown types in the macro
	// expansion. See `pezsp_session::runtime_api::*;` as one example. All runtime api decls should
	// be moved to file similarly.
	#[allow(ambiguous_glob_reexports)]
	pub mod apis {
		pub use pezframe_system_rpc_runtime_api::*;
		pub use pezsp_api::{self, *};
		pub use pezsp_block_builder::*;
		pub use pezsp_consensus_aura::*;
		pub use pezsp_consensus_grandpa::*;
		pub use pezsp_genesis_builder::*;
		pub use pezsp_offchain::*;
		pub use pezsp_session::runtime_api::*;
		pub use pezsp_transaction_pool::runtime_api::*;
	}

	/// A set of opinionated types aliases commonly used in runtimes.
	///
	/// This is one set of opinionated types. They are compatible with one another, but are not
	/// guaranteed to work if you start tweaking a portion.
	///
	/// Some note-worthy opinions in this prelude:
	///
	/// - `u32` block number.
	/// - [`pezsp_runtime::MultiAddress`] and [`pezsp_runtime::MultiSignature`] are used as the
	///   account id and signature types. This implies that this prelude can possibly used with an
	///   "account-index" system (eg `pezpallet-indices`). And, in any case, it should be paired
	///   with `AccountIdLookup` in [`pezframe_system::Config::Lookup`].
	pub mod types_common {
		use pezframe_system::Config as SysConfig;
		use pezsp_runtime::{generic, traits, OpaqueExtrinsic};

		/// A signature type compatible capably of handling multiple crypto-schemes.
		pub type Signature = pezsp_runtime::MultiSignature;

		/// The corresponding account-id type of [`Signature`].
		pub type AccountId =
			<<Signature as traits::Verify>::Signer as traits::IdentifyAccount>::AccountId;

		/// The block-number type, which should be fed into [`pezframe_system::Config`].
		pub type BlockNumber = u32;

		/// TODO: Ideally we want the hashing type to be equal to SysConfig::Hashing?
		type HeaderInner = generic::Header<BlockNumber, traits::BlakeTwo256>;

		// NOTE: `AccountIndex` is provided for future compatibility, if you want to introduce
		// something like `pezpallet-indices`.
		type ExtrinsicInner<T, Extra, AccountIndex = ()> = generic::UncheckedExtrinsic<
			pezsp_runtime::MultiAddress<AccountId, AccountIndex>,
			<T as SysConfig>::RuntimeCall,
			Signature,
			Extra,
		>;

		/// The block type, which should be fed into [`pezframe_system::Config`].
		///
		/// Should be parameterized with `T: pezframe_system::Config` and a tuple of
		/// `TransactionExtension`. When in doubt, use [`SystemTransactionExtensionsOf`].
		// Note that this cannot be dependent on `T` for block-number because it would lead to a
		// circular dependency (self-referential generics).
		pub type BlockOf<T, Extra = ()> = generic::Block<HeaderInner, ExtrinsicInner<T, Extra>>;

		/// The opaque block type. This is the same [`BlockOf`], but it has
		/// [`pezsp_runtime::OpaqueExtrinsic`] as its final extrinsic type.
		///
		/// This should be provided to the client side as the extrinsic type.
		pub type OpaqueBlock = generic::Block<HeaderInner, OpaqueExtrinsic>;

		/// Default set of signed extensions exposed from the `pezframe_system`.
		///
		/// crucially, this does NOT contain any tx-payment extension.
		pub type SystemTransactionExtensionsOf<T> = (
			pezframe_system::AuthorizeCall<T>,
			pezframe_system::CheckNonZeroSender<T>,
			pezframe_system::CheckSpecVersion<T>,
			pezframe_system::CheckTxVersion<T>,
			pezframe_system::CheckGenesis<T>,
			pezframe_system::CheckEra<T>,
			pezframe_system::CheckNonce<T>,
			pezframe_system::CheckWeight<T>,
			pezframe_system::WeightReclaim<T>,
		);
	}

	/// The main prelude of FRAME for building runtimes, and in the context of testing.
	///
	/// counter part of `runtime::prelude`.
	#[cfg(feature = "std")]
	pub mod testing_prelude {
		pub use pezsp_core::storage::Storage;
		pub use pezsp_runtime::{BuildStorage, DispatchError};
	}
}

/// All traits often used in FRAME pallets.
///
/// Note that types implementing these traits can also be found in this module.
// TODO: `Hash` and `Bounded` are defined multiple times; should be fixed once these two crates are
// cleaned up.
#[allow(ambiguous_glob_reexports)]
pub mod traits {
	pub use pezframe_support::traits::*;
	pub use pezsp_runtime::traits::*;
}

/// The arithmetic types used for safe math.
///
/// This is already part of the main [`prelude`].
pub mod arithmetic {
	pub use pezsp_arithmetic::{traits::*, *};
}

/// All token related types and traits.
pub mod token {
	pub use pezframe_support::traits::{
		tokens,
		tokens::{
			currency, fungible, fungibles, imbalance, nonfungible, nonfungible_v2, nonfungibles,
			nonfungibles_v2, pay, AssetId, BalanceStatus, DepositConsequence, ExistenceRequirement,
			Fortitude, Pay, Precision, Preservation, Provenance, WithdrawConsequence,
			WithdrawReasons,
		},
		OnUnbalanced,
	};
}

/// All derive macros used in frame.
///
/// This is already part of the main [`prelude`].
pub mod derive {
	pub use codec::{Decode, Encode};
	pub use core::fmt::Debug;
	pub use pezframe_support::{
		CloneNoBound, DebugNoBound, DefaultNoBound, EqNoBound, OrdNoBound, PartialEqNoBound,
		PartialOrdNoBound, RuntimeDebugNoBound,
	};
	pub use pezsp_runtime::RuntimeDebug;
	pub use scale_info::TypeInfo;
	pub use serde;
	/// The `serde` `Serialize`/`Deserialize` derive macros and traits.
	///
	/// You will either need to add `serde` as a dependency in your crate's `Cargo.toml`
	/// or specify the `#[serde(crate = "PATH_TO_THIS_CRATE::serde")]` attribute that points
	/// to the path where serde can be found.
	pub use serde::{Deserialize, Serialize};
}

/// All hashing related components.
///
/// This is already part of the main [`prelude`].
pub mod hashing {
	pub use pezsp_core::{hashing::*, H160, H256, H512, U256, U512};
	pub use pezsp_runtime::traits::{BlakeTwo256, Hash, Keccak256};
}

// Systems involved in transaction execution in the runtime.
///
/// This is already part of the [`prelude`].
pub mod transaction {
	pub use pezframe_support::traits::{CallMetadata, GetCallMetadata};
	pub use pezsp_runtime::{
		generic::ExtensionVersion,
		impl_tx_ext_default,
		traits::{
			AsTransactionAuthorizedOrigin, DispatchTransaction, TransactionExtension,
			ValidateResult,
		},
		transaction_validity::{InvalidTransaction, ValidTransaction},
	};
}

/// All account management related traits.
///
/// This is already part of the [`prelude`].
pub mod account {
	pub use pezframe_support::traits::{
		AsEnsureOriginWithArg, ChangeMembers, EitherOfDiverse, InitializeMembers,
	};
	pub use pezsp_runtime::traits::{IdentifyAccount, IdentityLookup};
}

/// Access to all of the dependencies of this crate. In case the prelude re-exports are not enough,
/// this module can be used.
///
/// Note: Before using these direct dependencies, please check if the item you need is available
/// through the preludes or domain-specific modules, as they provide a more organized and
/// maintainable way to access these dependencies.
pub mod deps {
	// Notes for maintainers: Any time one uses this module to access a dependency, you can have a
	// moment to think about whether this item could have been placed in any of the other modules
	// and preludes in this crate. In most cases, hopefully the answer is yes.

	pub use pezframe_support;
	pub use pezframe_system;

	pub use pezsp_arithmetic;
	pub use pezsp_core;
	pub use pezsp_io;
	pub use pezsp_runtime;

	pub use codec;
	pub use scale_info;

	#[cfg(feature = "runtime")]
	pub use pezframe_executive;
	#[cfg(feature = "runtime")]
	pub use pezsp_api;
	#[cfg(feature = "runtime")]
	pub use pezsp_block_builder;
	#[cfg(feature = "runtime")]
	pub use pezsp_consensus_aura;
	#[cfg(feature = "runtime")]
	pub use pezsp_consensus_grandpa;
	#[cfg(feature = "runtime")]
	pub use pezsp_genesis_builder;
	#[cfg(feature = "runtime")]
	pub use pezsp_inherents;
	#[cfg(feature = "runtime")]
	pub use pezsp_keyring;
	#[cfg(feature = "runtime")]
	pub use pezsp_offchain;
	#[cfg(feature = "runtime")]
	pub use pezsp_storage;
	#[cfg(feature = "runtime")]
	pub use pezsp_version;

	#[cfg(feature = "runtime-benchmarks")]
	pub use pezframe_benchmarking;
	#[cfg(feature = "runtime-benchmarks")]
	pub use pezframe_system_benchmarking;

	#[cfg(feature = "pezframe-try-runtime")]
	pub use pezframe_try_runtime;
}
