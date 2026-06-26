//! A shell pezpallet built with [`pezframe`].
//!
//! To get started with this pezpallet, try implementing the guide in
//! <https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html>
//!
//! [`pezframe`]: pezframe_support

#![cfg_attr(not(feature = "std"), no_std)]

// Re-export all pezpallet parts, this is needed to properly import the pezpallet into the runtime.
pub use pezpallet::*;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::storage]
	pub type Value<T> = StorageValue<_, u32>;
}
