// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
//! Governance API for controlling the Ethereum side of the bridge
//!
//! # Extrinsics
//!
//! ## Governance
//!
//! * [`Call::upgrade`]: Upgrade the Gateway contract on Ethereum.
//! * [`Call::set_operating_mode`]: Set the operating mode of the Gateway contract
//!
//! ## Pezkuwi-native tokens on Ethereum
//!
//! Tokens deposited on AssetHub pezpallet can be bridged to Ethereum as wrapped ERC20 tokens. As a
//! prerequisite, the token should be registered first.
//!
//! * [`Call::register_token`]: Register a token location as a wrapped ERC20 contract on Ethereum.
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod api;
pub mod weights;
pub use weights::*;

use pezframe_support::{pezpallet_prelude::*, traits::EnsureOrigin};
use pezframe_system::pezpallet_prelude::*;
pub use pezpallet::*;
use pezsnowbridge_core::{
	reward::{
		AddTip, MessageId,
		MessageId::{Inbound, Outbound},
	},
	AgentIdOf as LocationHashOf, AssetMetadata, TokenId, TokenIdOf,
};
use pezsnowbridge_outbound_queue_primitives::{
	v2::{Command, Initializer, Message, SendMessage},
	OperatingMode, SendError,
};
use pezsnowbridge_pezpallet_system::ForeignToNativeId;
use pezsp_core::{H160, H256};
use pezsp_io::hashing::blake2_256;
use pezsp_runtime::traits::MaybeConvert;
use pezsp_std::prelude::*;
use xcm::prelude::*;
use xcm_executor::traits::ConvertLocation;

#[cfg(feature = "runtime-benchmarks")]
use pezframe_support::traits::OriginTrait;

pub const LOG_TARGET: &str = "snowbridge-system-v2";

pub type AccountIdOf<T> = <T as pezframe_system::Config>::AccountId;
#[cfg(feature = "runtime-benchmarks")]
pub trait BenchmarkHelper<O>
where
	O: OriginTrait,
{
	fn make_xcm_origin(location: Location) -> O;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config + pezsnowbridge_pezpallet_system::Config {
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
		/// Send messages to Ethereum and add additional relayer rewards if deposited
		type OutboundQueue: SendMessage + AddTip;
		/// Add to the relayer reward for a specific message
		type InboundQueue: AddTip;
		/// Origin check for XCM locations that transact with this pezpallet
		type FrontendOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Location>;
		/// Origin for governance calls
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Location>;
		type WeightInfo: WeightInfo;
		#[cfg(feature = "runtime-benchmarks")]
		type Helper: BenchmarkHelper<Self::RuntimeOrigin>;
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An Upgrade message was sent to the Gateway
		Upgrade { impl_address: H160, impl_code_hash: H256, initializer_params_hash: H256 },
		/// An SetOperatingMode message was sent to the Gateway
		SetOperatingMode { mode: OperatingMode },
		/// Register Pezkuwi-native token as a wrapped ERC20 token on Ethereum
		RegisterToken {
			/// Location of Pezkuwi-native token
			location: VersionedLocation,
			/// ID of Pezkuwi-native token on Ethereum
			foreign_token_id: H256,
		},
		/// A tip was processed for an inbound or outbound message, for relayer incentivization.
		/// It could have succeeded or failed (and then added to LostTips).
		TipProcessed {
			/// The original sender of the tip (who deposited the funds).
			sender: AccountIdOf<T>,
			/// The Inbound/Outbound message nonce
			message_id: MessageId,
			/// The tip amount in ether.
			amount: u128,
			/// Whether the tip was added successfully. If the tip was added for a nonce
			/// that was already consumed, the tip will be added to LostTips.
			success: bool,
		},
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// Location could not be reachored
		LocationReanchorFailed,
		/// A token location could not be converted to a token ID.
		LocationConversionFailed,
		/// A `VersionedLocation` could not be converted into a `Location`.
		UnsupportedLocationVersion,
		/// An XCM could not be sent, due to a `SendError`.
		Send(SendError),
		/// The gateway contract upgrade message could not be sent due to invalid upgrade
		/// parameters.
		InvalidUpgradeParameters,
	}

	/// Relayer reward tips that were paid by the user to incentivize the processing of their
	/// message, but then could not be added to their message reward (e.g. the nonce was already
	/// processed or their order could not be found). Capturing the lost tips here supports
	/// implementing a recovery method in the future.
	#[pezpallet::storage]
	pub type LostTips<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, u128, ValueQuery>;

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Sends command to the Gateway contract to upgrade itself with a new implementation
		/// contract
		///
		/// Fee required: No
		///
		/// - `origin`: Must be `Root`.
		/// - `impl_address`: The address of the implementation contract.
		/// - `impl_code_hash`: The codehash of the implementation contract.
		/// - `initializer`: Optionally call an initializer on the implementation contract.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight((<T as pezpallet::Config>::WeightInfo::upgrade(), DispatchClass::Operational))]
		pub fn upgrade(
			origin: OriginFor<T>,
			impl_address: H160,
			impl_code_hash: H256,
			initializer: Initializer,
		) -> DispatchResult {
			let origin_location = T::GovernanceOrigin::ensure_origin(origin)?;
			let origin = Self::location_to_message_origin(origin_location)?;

			ensure!(
				!impl_address.eq(&H160::zero()) && !impl_code_hash.eq(&H256::zero()),
				Error::<T>::InvalidUpgradeParameters
			);

			let initializer_params_hash: H256 = blake2_256(initializer.params.as_ref()).into();

			let command = Command::Upgrade { impl_address, impl_code_hash, initializer };
			Self::send(origin, command, 0)?;

			Self::deposit_event(Event::<T>::Upgrade {
				impl_address,
				impl_code_hash,
				initializer_params_hash,
			});
			Ok(())
		}

		/// Sends a message to the Gateway contract to change its operating mode
		///
		/// Fee required: No
		///
		/// - `origin`: Must be `GovernanceOrigin`
		#[pezpallet::call_index(1)]
		#[pezpallet::weight((<T as pezpallet::Config>::WeightInfo::set_operating_mode(), DispatchClass::Operational))]
		pub fn set_operating_mode(origin: OriginFor<T>, mode: OperatingMode) -> DispatchResult {
			let origin_location = T::GovernanceOrigin::ensure_origin(origin)?;
			let origin = Self::location_to_message_origin(origin_location)?;

			let command = Command::SetOperatingMode { mode };
			Self::send(origin, command, 0)?;

			Self::deposit_event(Event::<T>::SetOperatingMode { mode });
			Ok(())
		}

		/// Registers a Pezkuwi-native token as a wrapped ERC20 token on Ethereum.
		///
		/// The system frontend pezpallet on AH proxies this call to BH.
		///
		/// - `sender`: The original sender initiating the call on AH
		/// - `asset_id`: Location of the asset (relative to this chain)
		/// - `metadata`: Metadata to include in the instantiated ERC20 contract on Ethereum
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::register_token())]
		pub fn register_token(
			origin: OriginFor<T>,
			sender: Box<VersionedLocation>,
			asset_id: Box<VersionedLocation>,
			metadata: AssetMetadata,
			amount: u128,
		) -> DispatchResult {
			T::FrontendOrigin::ensure_origin(origin)?;

			let sender_location: Location =
				(*sender).try_into().map_err(|_| Error::<T>::UnsupportedLocationVersion)?;
			let asset_location: Location =
				(*asset_id).try_into().map_err(|_| Error::<T>::UnsupportedLocationVersion)?;

			let location = Self::reanchor(asset_location)?;
			let token_id = TokenIdOf::convert_location(&location)
				.ok_or(Error::<T>::LocationConversionFailed)?;

			if !ForeignToNativeId::<T>::contains_key(token_id) {
				ForeignToNativeId::<T>::insert(token_id, location.clone());
			}

			let command = Command::RegisterForeignToken {
				token_id,
				name: metadata.name.into_inner(),
				symbol: metadata.symbol.into_inner(),
				decimals: metadata.decimals,
			};

			let message_origin = Self::location_to_message_origin(sender_location)?;
			Self::send(message_origin, command, amount)?;

			Self::deposit_event(Event::<T>::RegisterToken {
				location: location.into(),
				foreign_token_id: token_id,
			});

			Ok(())
		}

		#[pezpallet::call_index(3)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::add_tip())]
		pub fn add_tip(
			origin: OriginFor<T>,
			sender: AccountIdOf<T>,
			message_id: MessageId,
			amount: u128,
		) -> DispatchResult {
			T::FrontendOrigin::ensure_origin(origin)?;

			let result = match message_id {
				Inbound(nonce) => <T as pezpallet::Config>::InboundQueue::add_tip(nonce, amount),
				Outbound(nonce) => <T as pezpallet::Config>::OutboundQueue::add_tip(nonce, amount),
			};

			if let Err(ref e) = result {
				tracing::debug!(target: LOG_TARGET, ?e, ?message_id, ?amount, "error adding tip");
				LostTips::<T>::mutate(&sender, |lost_tip| {
					*lost_tip = lost_tip.saturating_add(amount);
				});
			}

			Self::deposit_event(Event::<T>::TipProcessed {
				sender,
				message_id,
				amount,
				success: result.is_ok(),
			});

			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		/// Send `command` to the Gateway from a specific origin/agent
		fn send(origin: H256, command: Command, fee: u128) -> DispatchResult {
			let message = Message {
				origin,
				id: pezframe_system::unique((origin, &command, fee)).into(),
				fee,
				commands: BoundedVec::try_from(vec![command]).unwrap(),
			};

			let ticket = <T as pezpallet::Config>::OutboundQueue::validate(&message)
				.map_err(|err| Error::<T>::Send(err))?;

			<T as pezpallet::Config>::OutboundQueue::deliver(ticket)
				.map_err(|err| Error::<T>::Send(err))?;
			Ok(())
		}

		/// Reanchor the `location` in context of ethereum
		pub fn reanchor(location: Location) -> Result<Location, Error<T>> {
			location
				.reanchored(&T::EthereumLocation::get(), &T::UniversalLocation::get())
				.map_err(|_| Error::<T>::LocationReanchorFailed)
		}

		pub fn location_to_message_origin(location: Location) -> Result<H256, Error<T>> {
			let reanchored_location = Self::reanchor(location)?;
			LocationHashOf::convert_location(&reanchored_location)
				.ok_or(Error::<T>::LocationConversionFailed)
		}
	}

	impl<T: Config> MaybeConvert<TokenId, Location> for Pezpallet<T> {
		fn maybe_convert(foreign_id: TokenId) -> Option<Location> {
			pezsnowbridge_pezpallet_system::Pezpallet::<T>::maybe_convert(foreign_id)
		}
	}
}
