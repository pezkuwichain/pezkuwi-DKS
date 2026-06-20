//! # Template Pezpallet
//!
//! A pezpallet with minimal functionality to help developers understand the essential components of
//! writing a FRAME pezpallet. It is typically used in beginner tutorials or in Pezkuwi SDK template
//! as a starting point for creating a new pezpallet and **not meant to be used in production**.
//!
//! ## Overview
//!
//! This template pezpallet contains basic examples of:
//! - declaring a storage item that stores a single block-number
//! - declaring and using events
//! - declaring and using errors
//! - a dispatchable function that allows a user to set a new value to storage and emits an event
//!   upon success
//! - another dispatchable function that causes a custom error to be thrown
//!
//! Each pezpallet section is annotated with an attribute using the `#[pezpallet::...]` procedural
//! macro. This macro generates the necessary code for a pezpallet to be aggregated into a FRAME
//! runtime.
//!
//! To get started with pezpallet development, consider using this tutorial:
//!
//! <https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html>
//!
//! And reading the main documentation of the `frame` crate:
//!
//! <https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html>
//!
//! And looking at the frame [`kitchen-sink`](https://docs.pezkuwichain.io/sdk/master/pezpallet_example_kitchensink/index.html)
//! pezpallet, a showcase of all pezpallet macros.
//!
//! ### Pezpallet Sections
//!
//! The pezpallet sections in this template are:
//!
//! - A **configuration trait** that defines the types and parameters which the pezpallet depends on
//!   (denoted by the `#[pezpallet::config]` attribute). See: [`Config`].
//! - A **means to store pezpallet-specific data** (denoted by the `#[pezpallet::storage]`
//!   attribute). See: [`storage_types`].
//! - A **declaration of the events** this pezpallet emits (denoted by the `#[pezpallet::event]`
//!   attribute). See: [`Event`].
//! - A **declaration of the errors** that this pezpallet can throw (denoted by the
//!   `#[pezpallet::error]` attribute). See: [`Error`].
//! - A **set of dispatchable functions** that define the pezpallet's functionality (denoted by the
//!   `#[pezpallet::call]` attribute). See: [`dispatchables`].
//!
//! Run `cargo doc --package pezpallet-template --open` to view this pezpallet's documentation.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pezpallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// <https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html>
// <https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html>
//
// To see a full list of `pezpallet` macros and their use cases, see:
// <https://docs.pezkuwichain.io/sdk/master/pezpallet_example_kitchensink/index.html>
// <https://docs.pezkuwichain.io/sdk/master/pezframe_support/pezpallet_macros/index.html>
#[pezframe::pezpallet]
pub mod pezpallet {
	use pezframe::prelude::*;

	/// Configure the pezpallet by specifying the parameters and types on which it depends.
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;

		/// A type representing the weights required by the dispatchables of this pezpallet.
		type WeightInfo: crate::weights::WeightInfo;
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	/// A struct to store a single block-number. Has all the right derives to store it in storage.
	/// <https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/reference_docs/frame_storage_derives/index.html>
	#[derive(
		Encode, Decode, MaxEncodedLen, TypeInfo, CloneNoBound, PartialEqNoBound, DefaultNoBound,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct CompositeStruct<T: Config> {
		/// A block number.
		pub(crate) block_number: BlockNumberFor<T>,
	}

	/// The pezpallet's storage items.
	/// <https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#storage>
	/// <https://docs.pezkuwichain.io/sdk/master/pezframe_support/pezpallet_macros/attr.storage.html>
	#[pezpallet::storage]
	pub type Something<T: Config> = StorageValue<_, CompositeStruct<T>>;

	/// Pallets use events to inform users when important changes are made.
	/// <https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error>
	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// We usually use passive tense for events.
		SomethingStored { block_number: BlockNumberFor<T>, who: T::AccountId },
	}

	/// Errors inform users that something went wrong.
	/// <https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error>
	#[pezpallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {}

	/// Dispatchable functions allows users to interact with the pezpallet and invoke state changes.
	/// These functions materialize as "extrinsics", which are often compared to transactions.
	/// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	/// <https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#dispatchables>
	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn do_something(origin: OriginFor<T>, bn: u32) -> DispatchResultWithPostInfo {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// <https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/reference_docs/frame_origin/index.html>
			let who = ensure_signed(origin)?;

			// Convert the u32 into a block number. This is possible because the set of trait bounds
			// defined in [`pezframe_system::Config::BlockNumber`].
			let block_number: BlockNumberFor<T> = bn.into();

			// Update storage.
			<Something<T>>::put(CompositeStruct { block_number });

			// Emit an event.
			Self::deposit_event(Event::SomethingStored { block_number, who });

			// Return a successful [`DispatchResultWithPostInfo`] or [`DispatchResult`].
			Ok(().into())
		}

		/// An example dispatchable that may throw a custom error.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1,1))]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValue)?,
				Some(mut old) => {
					// Increment the value read from storage; will error in the event of overflow.
					old.block_number = old
						.block_number
						.checked_add(&One::one())
						// ^^ equivalent is to:
						// .checked_add(&1u32.into())
						// both of which build a `One` instance for the type `BlockNumber`.
						.ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(old);
					// Explore how you can rewrite this using
					// [`pezframe_support::storage::StorageValue::mutate`].
					Ok(().into())
				},
			}
		}
	}
}
