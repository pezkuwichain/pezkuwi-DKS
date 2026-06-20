// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
//! Inbound Queue
//!
//! # Overview
//!
//! Receives messages emitted by the Gateway contract on Ethereum, whereupon they are verified,
//! translated to XCM, and finally sent to their final destination teyrchain.
//!
//! The message relayers are rewarded using native currency from the sovereign account of the
//! destination teyrchain.
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
//! * [`Call::submit`]: Submit a message for verification and dispatch the final destination
//!   teyrchain.
#![cfg_attr(not(feature = "std"), no_std)]

mod envelope;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

use codec::{Decode, DecodeAll, Encode};
use envelope::Envelope;
use pezframe_support::{
	traits::{
		fungible::{Inspect, Mutate},
		tokens::{Fortitude, Preservation},
	},
	weights::WeightToFee,
	PalletError,
};
use pezframe_system::ensure_signed;
use pezsp_core::H160;
use pezsp_runtime::traits::Zero;
use scale_info::TypeInfo;
use xcm::prelude::{
	send_xcm, Junction::*, Location, SendError as XcmpSendError, SendXcm, Xcm, XcmContext, XcmHash,
};
use xcm_executor::traits::TransactAsset;

use pezsnowbridge_core::{
	sibling_sovereign_account, BasicOperatingMode, Channel, ChannelId, ParaId, PricingParameters,
	StaticLookup,
};
use pezsnowbridge_inbound_queue_primitives::{
	v1::{ConvertMessage, ConvertMessageError, VersionedMessage},
	EventProof, VerificationError, Verifier,
};

use pezsp_runtime::{traits::Saturating, SaturatedConversion, TokenError};

pub use weights::WeightInfo;

type BalanceOf<T> = <<T as pezpallet::Config>::Token as Inspect<
	<T as pezframe_system::Config>::AccountId,
>>::Balance;

pub use pezpallet::*;

pub const LOG_TARGET: &str = "snowbridge-inbound-queue";

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_core::H256;

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

		/// The verifier for inbound messages from Ethereum
		type Verifier: Verifier;

		/// Message relayers are rewarded with this asset
		type Token: Mutate<Self::AccountId> + Inspect<Self::AccountId>;

		/// XCM message sender
		type XcmSender: SendXcm;

		// Address of the Gateway contract
		#[pezpallet::constant]
		type GatewayAddress: Get<H160>;

		/// Convert inbound message to XCM
		type MessageConverter: ConvertMessage<
			AccountId = Self::AccountId,
			Balance = BalanceOf<Self>,
		>;

		/// Lookup a channel descriptor
		type ChannelLookup: StaticLookup<Source = ChannelId, Target = Channel>;

		/// Lookup pricing parameters
		type PricingParameters: Get<PricingParameters<BalanceOf<Self>>>;

		type WeightInfo: WeightInfo;

		#[cfg(feature = "runtime-benchmarks")]
		type Helper: BenchmarkHelper<Self>;

		/// Convert a weight value into deductible balance type.
		type WeightToFee: WeightToFee<Balance = BalanceOf<Self>>;

		/// Convert a length value into deductible balance type
		type LengthToFee: WeightToFee<Balance = BalanceOf<Self>>;

		/// The upper limit here only used to estimate delivery cost
		type MaxMessageSize: Get<u32>;

		/// To withdraw and deposit an asset.
		type AssetTransactor: TransactAsset;
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A message was received from Ethereum
		MessageReceived {
			/// The message channel
			channel_id: ChannelId,
			/// The message nonce
			nonce: u64,
			/// ID of the XCM message which was forwarded to the final destination teyrchain
			message_id: [u8; 32],
			/// Fee burned for the teleport
			fee_burned: BalanceOf<T>,
		},
		/// Set OperatingMode
		OperatingModeChanged { mode: BasicOperatingMode },
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// Message came from an invalid outbound channel on the Ethereum side.
		InvalidGateway,
		/// Message has an invalid envelope.
		InvalidEnvelope,
		/// Message has an unexpected nonce.
		InvalidNonce,
		/// Message has an invalid payload.
		InvalidPayload,
		/// Message channel is invalid
		InvalidChannel,
		/// The max nonce for the type has been reached
		MaxNonceReached,
		/// Cannot convert location
		InvalidAccountConversion,
		/// Pezpallet is halted
		Halted,
		/// Message verification error,
		Verification(VerificationError),
		/// XCMP send failure
		Send(SendError),
		/// Message conversion error
		ConvertMessage(ConvertMessageError),
	}

	#[derive(
		Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, PalletError,
	)]
	pub enum SendError {
		NotApplicable,
		NotRoutable,
		Transport,
		DestinationUnsupported,
		ExceedsMaxMessageSize,
		MissingArgument,
		Fees,
	}

	impl<T: Config> From<XcmpSendError> for Error<T> {
		fn from(e: XcmpSendError) -> Self {
			match e {
				XcmpSendError::NotApplicable => Error::<T>::Send(SendError::NotApplicable),
				XcmpSendError::Unroutable => Error::<T>::Send(SendError::NotRoutable),
				XcmpSendError::Transport(_) => Error::<T>::Send(SendError::Transport),
				XcmpSendError::DestinationUnsupported => {
					Error::<T>::Send(SendError::DestinationUnsupported)
				},
				XcmpSendError::ExceedsMaxMessageSize => {
					Error::<T>::Send(SendError::ExceedsMaxMessageSize)
				},
				XcmpSendError::MissingArgument => Error::<T>::Send(SendError::MissingArgument),
				XcmpSendError::Fees => Error::<T>::Send(SendError::Fees),
			}
		}
	}

	/// The current nonce for each channel
	#[pezpallet::storage]
	pub type Nonce<T: Config> = StorageMap<_, Twox64Concat, ChannelId, u64, ValueQuery>;

	/// The current operating mode of the pezpallet.
	#[pezpallet::storage]
	#[pezpallet::getter(fn operating_mode)]
	pub type OperatingMode<T: Config> = StorageValue<_, BasicOperatingMode, ValueQuery>;

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Submit an inbound message originating from the Gateway contract on Ethereum
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::submit())]
		pub fn submit(origin: OriginFor<T>, event: EventProof) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!Self::operating_mode().is_halted(), Error::<T>::Halted);

			// submit message to verifier for verification
			T::Verifier::verify(&event.event_log, &event.proof)
				.map_err(|e| Error::<T>::Verification(e))?;

			// Decode event log into an Envelope
			let envelope =
				Envelope::try_from(&event.event_log).map_err(|_| Error::<T>::InvalidEnvelope)?;

			// Verify that the message was submitted from the known Gateway contract
			ensure!(T::GatewayAddress::get() == envelope.gateway, Error::<T>::InvalidGateway);

			// Retrieve the registered channel for this message
			let channel =
				T::ChannelLookup::lookup(envelope.channel_id).ok_or(Error::<T>::InvalidChannel)?;

			// Verify message nonce
			<Nonce<T>>::try_mutate(envelope.channel_id, |nonce| -> DispatchResult {
				if *nonce == u64::MAX {
					return Err(Error::<T>::MaxNonceReached.into());
				}
				if envelope.nonce != nonce.saturating_add(1) {
					Err(Error::<T>::InvalidNonce.into())
				} else {
					*nonce = nonce.saturating_add(1);
					Ok(())
				}
			})?;

			// Reward relayer from the sovereign account of the destination teyrchain, only if funds
			// are available
			let sovereign_account = sibling_sovereign_account::<T>(channel.para_id);
			let delivery_cost = Self::calculate_delivery_cost(event.encode().len() as u32);
			let amount = T::Token::reducible_balance(
				&sovereign_account,
				Preservation::Preserve,
				Fortitude::Polite,
			)
			.min(delivery_cost);
			if !amount.is_zero() {
				T::Token::transfer(&sovereign_account, &who, amount, Preservation::Preserve)?;
			}

			// Decode payload into `VersionedMessage`
			let message = VersionedMessage::decode_all(&mut envelope.payload.as_ref())
				.map_err(|_| Error::<T>::InvalidPayload)?;

			// Decode message into XCM
			let (xcm, fee) = Self::do_convert(envelope.message_id, message.clone())?;

			tracing::info!(
				target: LOG_TARGET,
				?xcm,
				?fee,
				"💫 xcm decoded"
			);

			// Burning fees for teleport
			Self::burn_fees(channel.para_id, fee)?;

			// Attempt to send XCM to a dest teyrchain
			let message_id = Self::send_xcm(xcm, channel.para_id)?;

			Self::deposit_event(Event::MessageReceived {
				channel_id: envelope.channel_id,
				nonce: envelope.nonce,
				message_id,
				fee_burned: fee,
			});

			Ok(())
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
		pub fn do_convert(
			message_id: H256,
			message: VersionedMessage,
		) -> Result<(Xcm<()>, BalanceOf<T>), Error<T>> {
			let (xcm, fee) = T::MessageConverter::convert(message_id, message)
				.map_err(|e| Error::<T>::ConvertMessage(e))?;
			Ok((xcm, fee))
		}

		pub fn send_xcm(xcm: Xcm<()>, dest: ParaId) -> Result<XcmHash, Error<T>> {
			let dest = Location::new(1, [Teyrchain(dest.into())]);
			let (xcm_hash, _) = send_xcm::<T::XcmSender>(dest, xcm).map_err(Error::<T>::from)?;
			Ok(xcm_hash)
		}

		pub fn calculate_delivery_cost(length: u32) -> BalanceOf<T> {
			let weight_fee = T::WeightToFee::weight_to_fee(&T::WeightInfo::submit());
			let len_fee = T::LengthToFee::weight_to_fee(&Weight::from_parts(length as u64, 0));
			weight_fee
				.saturating_add(len_fee)
				.saturating_add(T::PricingParameters::get().rewards.local)
		}

		/// Burn the amount of the fee embedded into the XCM for teleports
		pub fn burn_fees(para_id: ParaId, fee: BalanceOf<T>) -> DispatchResult {
			let dummy_context =
				XcmContext { origin: None, message_id: Default::default(), topic: None };
			let dest = Location::new(1, [Teyrchain(para_id.into())]);
			let fees = (Location::parent(), fee.saturated_into::<u128>()).into();
			T::AssetTransactor::can_check_out(&dest, &fees, &dummy_context).map_err(|error| {
				tracing::error!(
					target: LOG_TARGET,
					?error,
					"XCM asset check out failed with error"
				);
				TokenError::FundsUnavailable
			})?;
			T::AssetTransactor::check_out(&dest, &fees, &dummy_context);
			T::AssetTransactor::withdraw_asset(&fees, &dest, None).map_err(|error| {
				tracing::error!(
					target: LOG_TARGET,
					?error,
					"XCM asset withdraw failed with error"
				);
				TokenError::FundsUnavailable
			})?;
			Ok(())
		}
	}

	/// API for accessing the delivery cost of a message
	impl<T: Config> Get<BalanceOf<T>> for Pezpallet<T> {
		fn get() -> BalanceOf<T> {
			// Cost here based on MaxMessagePayloadSize(the worst case)
			Self::calculate_delivery_cost(T::MaxMessageSize::get())
		}
	}
}
