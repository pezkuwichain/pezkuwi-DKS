// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
//! Inbound Queue
//!
//! # Overview
//!
//! Receives messages emitted by the Gateway contract on Ethereum, whereupon they are verified,
//! translated to XCM, and finally sent to AssetHub for further processing.
//!
//! Message relayers are rewarded in wrapped Ether that is included within the message. This
//! wrapped Ether is derived from Ether that the message origin has locked up on Ethereum.
//!
//! # Extrinsics
//!
//! ## Governance
//!
//! * [`Call::set_operating_mode`]: Set the operating mode of the pezpallet. Can be used to disable
//!   processing of inbound messages.
//!
//! ## Message Submission
//!
//! * [`Call::submit`]: Submit a message for verification and dispatch to the final destination
//!   teyrchain.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub use crate::weights::WeightInfo;
use pezbp_relayers::RewardLedger;
use pezframe_system::ensure_signed;
use pezsnowbridge_core::{
	reward::{AddTip, AddTipError},
	sparse_bitmap::{SparseBitmap, SparseBitmapImpl},
	BasicOperatingMode,
};
use pezsnowbridge_inbound_queue_primitives::{
	v2::{ConvertMessage, ConvertMessageError, Message},
	EventProof, VerificationError, Verifier,
};
use pezsp_core::H160;
use pezsp_runtime::traits::TryConvert;
use pezsp_std::prelude::*;
use xcm::prelude::{ExecuteXcm, Junction::*, Location, SendXcm, *};

pub use pezpallet::*;

pub const LOG_TARGET: &str = "pezsnowbridge-pezpallet-inbound-queue-v2";

pub type AccountIdOf<T> = <T as pezframe_system::Config>::AccountId;

pub type Nonce<T> = SparseBitmapImpl<crate::NonceBitmap<T>>;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	#[cfg(feature = "runtime-benchmarks")]
	use pezsnowbridge_inbound_queue_primitives::EventFixture;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[cfg(feature = "runtime-benchmarks")]
	pub trait BenchmarkHelper<T> {
		fn initialize_storage() -> EventFixture;
	}

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
		/// The verifier for inbound messages from Ethereum.
		type Verifier: Verifier;
		/// XCM message sender.
		type XcmSender: SendXcm;
		/// Handler for XCM fees.
		type XcmExecutor: ExecuteXcm<Self::RuntimeCall>;
		/// Address of the Gateway contract.
		#[pezpallet::constant]
		type GatewayAddress: Get<H160>;
		/// AssetHub teyrchain ID.
		type AssetHubParaId: Get<u32>;
		/// Convert a command from Ethereum to an XCM message.
		type MessageConverter: ConvertMessage;
		#[cfg(feature = "runtime-benchmarks")]
		type Helper: BenchmarkHelper<Self>;
		/// Reward discriminator type.
		type RewardKind: Parameter + MaxEncodedLen + Send + Sync + Copy + Clone;
		/// The default RewardKind discriminator for rewards allocated to relayers from this
		/// pezpallet.
		#[pezpallet::constant]
		type DefaultRewardKind: Get<Self::RewardKind>;
		/// Relayer reward payment.
		type RewardPayment: RewardLedger<Self::AccountId, Self::RewardKind, u128>;
		/// AccountId to Location converter
		type AccountToLocation: for<'a> TryConvert<&'a Self::AccountId, Location>;
		type WeightInfo: WeightInfo;
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A message was received from Ethereum
		MessageReceived {
			/// The message nonce
			nonce: u64,
			/// ID of the XCM message which was forwarded to the final destination teyrchain
			message_id: [u8; 32],
		},
		/// Set OperatingMode
		OperatingModeChanged { mode: BasicOperatingMode },
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// Message came from an invalid outbound channel on the Ethereum side.
		InvalidGateway,
		/// Account could not be converted to bytes
		InvalidAccount,
		/// Message has an invalid envelope.
		InvalidMessage,
		/// Message has an unexpected nonce.
		InvalidNonce,
		/// Fee provided is invalid.
		InvalidFee,
		/// Message has an invalid payload.
		InvalidPayload,
		/// Message channel is invalid
		InvalidChannel,
		/// The max nonce for the type has been reached
		MaxNonceReached,
		/// Cannot convert location
		InvalidAccountConversion,
		/// Invalid network specified
		InvalidNetwork,
		/// Pezpallet is halted
		Halted,
		/// The operation required fees to be paid which the initiator could not meet.
		FeesNotMet,
		/// The desired destination was unreachable, generally because there is a no way of routing
		/// to it.
		Unreachable,
		/// There was some other issue (i.e. not to do with routing) in sending the message.
		/// Perhaps a lack of space for buffering the message.
		SendFailure,
		/// Invalid foreign ERC-20 token ID
		InvalidAsset,
		/// Cannot reachor a foreign ERC-20 asset location.
		CannotReanchor,
		/// Message verification error
		Verification(VerificationError),
	}

	impl<T: Config> From<SendError> for Error<T> {
		fn from(e: SendError) -> Self {
			match e {
				SendError::Fees => Error::<T>::FeesNotMet,
				SendError::NotApplicable => Error::<T>::Unreachable,
				_ => Error::<T>::SendFailure,
			}
		}
	}

	impl<T: Config> From<ConvertMessageError> for Error<T> {
		fn from(e: ConvertMessageError) -> Self {
			match e {
				ConvertMessageError::InvalidAsset => Error::<T>::InvalidAsset,
				ConvertMessageError::CannotReanchor => Error::<T>::CannotReanchor,
				ConvertMessageError::InvalidNetwork => Error::<T>::InvalidNetwork,
			}
		}
	}

	/// StorageMap used for encoding a SparseBitmapImpl that tracks whether a specific nonce has
	/// been processed or not. Message nonces are unique and never repeated.
	#[pezpallet::storage]
	pub type NonceBitmap<T: Config> = StorageMap<_, Twox64Concat, u64, u128, ValueQuery>;

	/// The current operating mode of the pezpallet.
	#[pezpallet::storage]
	pub type OperatingMode<T: Config> = StorageValue<_, BasicOperatingMode, ValueQuery>;

	/// Keep track of tips added for a message as an additional relayer incentivization. The
	/// key for the storage map is the nonce of the message to which the tip should be added.
	/// The value is the tip amount, in Ether.
	#[pezpallet::storage]
	pub type Tips<T: Config> = StorageMap<_, Blake2_128Concat, u64, u128, OptionQuery>;

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Submit an inbound message originating from the Gateway contract on Ethereum
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::submit())]
		pub fn submit(origin: OriginFor<T>, event: Box<EventProof>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!OperatingMode::<T>::get().is_halted(), Error::<T>::Halted);

			// submit message for verification
			T::Verifier::verify(&event.event_log, &event.proof)
				.map_err(|e| Error::<T>::Verification(e))?;

			// Decode event log into a bridge message
			let message =
				Message::try_from(&event.event_log).map_err(|_| Error::<T>::InvalidMessage)?;

			Self::process_message(who, message)
		}

		/// Halt or resume all pezpallet operations. May only be called by root.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight((T::DbWeight::get().reads_writes(1, 1), DispatchClass::Operational))]
		pub fn set_operating_mode(
			origin: OriginFor<T>,
			mode: BasicOperatingMode,
		) -> DispatchResult {
			ensure_root(origin)?;
			OperatingMode::<T>::set(mode);
			Self::deposit_event(Event::OperatingModeChanged { mode });
			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		pub fn process_message(relayer: T::AccountId, message: Message) -> DispatchResult {
			// Verify that the message was submitted from the known Gateway contract
			ensure!(T::GatewayAddress::get() == message.gateway, Error::<T>::InvalidGateway);

			let (nonce, relayer_fee) = (message.nonce, message.relayer_fee);

			// Verify the message has not been processed
			ensure!(!Nonce::<T>::get(nonce), Error::<T>::InvalidNonce);

			let xcm =
				T::MessageConverter::convert(message).map_err(|error| Error::<T>::from(error))?;

			// Forward XCM to AH
			let dest = Location::new(1, [Teyrchain(T::AssetHubParaId::get())]);

			// Mark message as received
			Nonce::<T>::set(nonce);

			let message_id =
				Self::send_xcm(dest.clone(), &relayer, xcm.clone()).map_err(|error| {
					tracing::error!(target: LOG_TARGET, ?error, ?dest, ?xcm, "XCM send failed with error");
					Error::<T>::from(error)
				})?;

			// Pay relayer reward
			let tip = Tips::<T>::take(nonce).unwrap_or_default();
			let total_tip = relayer_fee.saturating_add(tip);
			if total_tip > 0 {
				T::RewardPayment::register_reward(&relayer, T::DefaultRewardKind::get(), total_tip);
			}

			Self::deposit_event(Event::MessageReceived { nonce, message_id });

			Ok(())
		}

		fn send_xcm(
			dest: Location,
			fee_payer: &T::AccountId,
			xcm: Xcm<()>,
		) -> Result<XcmHash, SendError> {
			let (ticket, fee) = validate_send::<T::XcmSender>(dest, xcm)?;
			let fee_payer = T::AccountToLocation::try_convert(fee_payer).map_err(|err| {
				tracing::error!(
					target: LOG_TARGET,
					?err,
					"Failed to convert account to XCM location",
				);
				SendError::NotApplicable
			})?;
			T::XcmExecutor::charge_fees(fee_payer.clone(), fee.clone()).map_err(|error| {
				tracing::error!(
					target: LOG_TARGET,
					?error,
					"Charging fees failed with error",
				);
				SendError::Fees
			})?;
			T::XcmSender::deliver(ticket)
		}
	}

	impl<T: Config> AddTip for Pezpallet<T> {
		fn add_tip(nonce: u64, amount: u128) -> Result<(), AddTipError> {
			ensure!(amount > 0, AddTipError::AmountZero);
			// If the nonce is already processed, return an error
			ensure!(!Nonce::<T>::get(nonce.into()), AddTipError::NonceConsumed);
			// Otherwise add the tip.
			Tips::<T>::mutate(nonce, |tip| {
				*tip = Some(tip.unwrap_or_default().saturating_add(amount));
			});
			return Ok(());
		}
	}
}
