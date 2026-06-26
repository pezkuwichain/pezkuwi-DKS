//! # Template Pezpallet
//!
//! A pezpallet with minimal functionality to help developers understand the essential components of
//! writing a FRAME pezpallet. It is typically used in beginner tutorials or in Bizinikiwi template
//! nodes as a starting point for creating a new pezpallet and **not meant to be used in
//! production**.
//!
//! ## Overview
//!
//! This template pezpallet contains basic examples of:
//! - declaring a storage item that stores a single `u32` value
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
//! Learn more about FRAME macros [here](https://docs.pezkuwichain.io/reference/frame-macros/).
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

// We make sure this pezpallet uses `no_std` for compiling to Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pezpallet items so that they can be accessed from the crate namespace.
pub use pezpallet::*;

// FRAME pallets require their own "mock runtimes" to be able to run unit tests. This module
// contains a mock runtime specific for testing this pezpallet's functionality.
#[cfg(test)]
mod mock;

// This module contains the unit tests for this pezpallet.
// Learn about pezpallet unit testing here: https://docs.pezkuwichain.io/test/unit-testing/
#[cfg(test)]
mod tests;

// Every callable function or "dispatchable" a pezpallet exposes must have weight values that
// correctly estimate a dispatchable's execution time. The benchmarking module is used to calculate
// weights for each dispatchable and generates this pezpallet's weight.rs file. Learn more about benchmarking here: https://docs.pezkuwichain.io/test/benchmark/
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

// All pezpallet logic is defined in its own module and must be annotated by the `pezpallet`
// attribute.
#[pezframe_support::pezpallet]
pub mod pezpallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	// The `Pezpallet` struct serves as a placeholder to implement traits, methods and dispatchables
	// (`Call`s) in this pezpallet.
	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	/// The pezpallet's configuration trait.
	///
	/// All our types and constants a pezpallet depends on must be declared here.
	/// These types are defined generically and made concrete when the pezpallet is declared in the
	/// `runtime/src/lib.rs` file of your chain.
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		/// The overarching runtime event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
		/// A type representing the weights required by the dispatchables of this pezpallet.
		type WeightInfo: WeightInfo;
	}

	/// A storage item for this pezpallet.
	///
	/// In this template, we are declaring a storage item called `Something` that stores a single
	/// `u32` value. Learn more about runtime storage here: <https://docs.pezkuwichain.io/build/runtime-storage/>
	#[pezpallet::storage]
	pub type Something<T> = StorageValue<_, u32>;

	/// Events that functions in this pezpallet can emit.
	///
	/// Events are a simple means of indicating to the outside world (such as dApps, chain explorers
	/// or other users) that some notable update in the runtime has occurred. In a FRAME pezpallet,
	/// the documentation for each event field and its parameters is added to a node's metadata so
	/// it can be used by external interfaces or tools.
	///
	///	The `generate_deposit` macro generates a function on `Pezpallet` called `deposit_event`
	/// which will convert the event type of your pezpallet into `RuntimeEvent` (declared in the
	/// pezpallet's [`Config`] trait) and deposit it using
	/// [`pezframe_system::Pezpallet::deposit_event`].
	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A user has successfully set a new value.
		SomethingStored {
			/// The new value set.
			something: u32,
			/// The account who set the new value.
			who: T::AccountId,
		},
	}

	/// Errors that can be returned by this pezpallet.
	///
	/// Errors tell users that something went wrong so it's important that their naming is
	/// informative. Similar to events, error documentation is added to a node's metadata so it's
	/// equally important that they have helpful documentation associated with them.
	///
	/// This type of runtime error can be up to 4 bytes in size should you want to return additional
	/// information.
	#[pezpallet::error]
	pub enum Error<T> {
		/// The value retrieved was `None` as no value was previously set.
		NoneValue,
		/// There was an attempt to increment the value in storage over `u32::MAX`.
		StorageOverflow,
	}

	/// The pezpallet's dispatchable functions ([`Call`]s).
	///
	/// Dispatchable functions allows users to interact with the pezpallet and invoke state changes.
	/// These functions materialize as "extrinsics", which are often compared to transactions.
	/// They must always return a `DispatchResult` and be annotated with a weight and call index.
	///
	/// The [`call_index`] macro is used to explicitly
	/// define an index for calls in the [`Call`] enum. This is useful for pallets that may
	/// introduce new dispatchables over time. If the order of a dispatchable changes, its index
	/// will also change which will break backwards compatibility.
	///
	/// The [`weight`] macro is used to assign a weight to each call.
	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// An example dispatchable that takes a single u32 value as a parameter, writes the value
		/// to storage and emits an event.
		///
		/// It checks that the _origin_ for this call is _Signed_ and returns a dispatch
		/// error if it isn't. Learn more about origins here: <https://docs.pezkuwichain.io/build/origins/>
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::do_something())]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			let who = ensure_signed(origin)?;

			// Update storage.
			Something::<T>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored { something, who });

			// Return a successful `DispatchResult`
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		///
		/// It checks that the caller is a signed origin and reads the current value from the
		/// `Something` storage item. If a current value exists, it is incremented by 1 and then
		/// written back to storage.
		///
		/// ## Errors
		///
		/// The function will return an error under the following conditions:
		///
		/// - If no value has been set ([`Error::NoneValue`])
		/// - If incrementing the value in storage causes an arithmetic overflow
		///   ([`Error::StorageOverflow`])
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::cause_error())]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match Something::<T>::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValue.into()),
				Some(old) => {
					// Increment the value read from storage. This will cause an error in the event
					// of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					Something::<T>::put(new);
					Ok(())
				},
			}
		}
	}
}
