#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod apis;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarks;
pub mod configs;

extern crate alloc;
use alloc::vec::Vec;
use pezsp_runtime::{
	generic, impl_opaque_keys,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiAddress, MultiSignature,
};
#[cfg(feature = "std")]
use pezsp_version::NativeVersion;
use pezsp_version::RuntimeVersion;

pub use pezframe_system::Call as SystemCall;
pub use pezpallet_balances::Call as BalancesCall;
pub use pezpallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use pezsp_runtime::BuildStorage;

pub mod genesis_config_presets;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	use pezsp_runtime::{
		generic,
		traits::{BlakeTwo256, Hash as HashT},
	};

	pub use pezsp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
	/// Opaque block hash type.
	pub type Hash = <BlakeTwo256 as HashT>::Output;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
		pub grandpa: Grandpa,
	}
}

// To learn more about runtime versioning, see:
// https://docs.pezkuwichain.io/main-docs/build/upgrade#runtime-versioning
#[pezsp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("pez-solochain-template-runtime"),
	impl_name: alloc::borrow::Cow::Borrowed("pez-solochain-template-runtime"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Pezkuwi-JS App (https://pezkuwichain.io) to use
	//   the compatible custom types.
	spec_version: 100,
	impl_version: 1,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

mod block_times {
	/// This determines the average expected block time that we are targeting. Blocks will be
	/// produced at a minimum duration defined by `SLOT_DURATION`. `SLOT_DURATION` is picked up by
	/// `pezpallet_timestamp` which is in turn picked up by `pezpallet_aura` to implement `fn
	/// slot_duration()`.
	///
	/// Change this to adjust the block time.
	pub const MILLI_SECS_PER_BLOCK: u64 = 6000;

	// NOTE: Currently it is not possible to change the slot duration after the chain has started.
	// Attempting to do so will brick block production.
	pub const SLOT_DURATION: u64 = MILLI_SECS_PER_BLOCK;
}
pub use block_times::*;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLI_SECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const BLOCK_HASH_COUNT: BlockNumber = 2400;

// Unit = the base number of indivisible units for balances
pub const UNIT: Balance = 1_000_000_000_000;
pub const MILLI_UNIT: Balance = 1_000_000_000;
pub const MICRO_UNIT: Balance = 1_000_000;

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: Balance = MILLI_UNIT;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = pezsp_core::H256;

/// An index to a block.
pub type BlockNumber = u32;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The `TransactionExtension` to the basic transaction logic.
pub type TxExtension = (
	pezframe_system::AuthorizeCall<Runtime>,
	pezframe_system::CheckNonZeroSender<Runtime>,
	pezframe_system::CheckSpecVersion<Runtime>,
	pezframe_system::CheckTxVersion<Runtime>,
	pezframe_system::CheckGenesis<Runtime>,
	pezframe_system::CheckEra<Runtime>,
	pezframe_system::CheckNonce<Runtime>,
	pezframe_system::CheckWeight<Runtime>,
	pezpallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	pezframe_metadata_hash_extension::CheckMetadataHash<Runtime>,
	pezframe_system::WeightReclaim<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, TxExtension>;

/// Executive: handles dispatch to the various modules.
pub type Executive = pezframe_executive::Executive<
	Runtime,
	Block,
	pezframe_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

// Create the runtime by composing the FRAME pallets that were previously configured.
#[pezframe_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Runtime;

	#[runtime::pezpallet_index(0)]
	pub type System = pezframe_system;

	#[runtime::pezpallet_index(1)]
	pub type Timestamp = pezpallet_timestamp;

	#[runtime::pezpallet_index(2)]
	pub type Aura = pezpallet_aura;

	#[runtime::pezpallet_index(3)]
	pub type Grandpa = pezpallet_grandpa;

	#[runtime::pezpallet_index(4)]
	pub type Balances = pezpallet_balances;

	#[runtime::pezpallet_index(5)]
	pub type TransactionPayment = pezpallet_transaction_payment;

	#[runtime::pezpallet_index(6)]
	pub type Sudo = pezpallet_sudo;

	// Include the custom logic from the pezpallet-template in the runtime.
	#[runtime::pezpallet_index(7)]
	pub type Template = pezpallet_template;
}
