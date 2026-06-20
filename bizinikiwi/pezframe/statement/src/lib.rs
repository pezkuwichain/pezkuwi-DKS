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

//! Supporting pezpallet for the statement store.
//!
//! - [`Pezpallet`]
//!
//! ## Overview
//!
//! The Statement pezpallet provides means to create and validate statements for the statement
//! store.
//!
//! For each statement validation function calculates the following three values based on the
//! statement author balance:
//! `max_count`: Maximum number of statements allowed for the author (signer) of this statement.
//! `max_size`: Maximum total size of statements allowed for the author (signer) of this statement.
//!
//! This pezpallet also contains an offchain worker that turns on-chain statement events into
//! statements. These statements are placed in the store and propagated over the network.

#![cfg_attr(not(feature = "std"), no_std)]

use pezframe_support::{
	pezpallet_prelude::*,
	pezsp_runtime::{traits::CheckedDiv, SaturatedConversion},
	traits::fungible::Inspect,
};
use pezframe_system::pezpallet_prelude::*;
use pezsp_statement_store::{
	runtime_api::{InvalidStatement, StatementSource, ValidStatement},
	Proof, SignatureVerificationResult, Statement,
};

#[cfg(test)]
// We do not declare all features used by `construct_runtime`
#[allow(unexpected_cfgs)]
mod mock;
#[cfg(test)]
mod tests;

pub use pezpallet::*;

const LOG_TARGET: &str = "runtime::statement";

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Inspect<<T as pezframe_system::Config>::AccountId>>::Balance;

	pub type AccountIdOf<T> = <T as pezframe_system::Config>::AccountId;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config
	where
		<Self as pezframe_system::Config>::AccountId: From<pezsp_statement_store::AccountId>,
	{
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
		/// The currency which is used to calculate account limits.
		type Currency: Inspect<Self::AccountId>;
		/// Min balance for priority statements.
		#[pezpallet::constant]
		type StatementCost: Get<BalanceOf<Self>>;
		/// Cost of data byte used for priority calculation.
		#[pezpallet::constant]
		type ByteCost: Get<BalanceOf<Self>>;
		/// Minimum number of statements allowed per account.
		#[pezpallet::constant]
		type MinAllowedStatements: Get<u32>;
		/// Maximum number of statements allowed per account.
		#[pezpallet::constant]
		type MaxAllowedStatements: Get<u32>;
		/// Minimum data bytes allowed per account.
		#[pezpallet::constant]
		type MinAllowedBytes: Get<u32>;
		/// Maximum data bytes allowed per account.
		#[pezpallet::constant]
		type MaxAllowedBytes: Get<u32>;
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(core::marker::PhantomData<T>);

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config>
	where
		<T as pezframe_system::Config>::AccountId: From<pezsp_statement_store::AccountId>,
	{
		/// A new statement is submitted
		NewStatement { account: T::AccountId, statement: Statement },
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T>
	where
		<T as pezframe_system::Config>::AccountId: From<pezsp_statement_store::AccountId>,
		pezsp_statement_store::AccountId: From<<T as pezframe_system::Config>::AccountId>,
		<T as pezframe_system::Config>::RuntimeEvent: From<pezpallet::Event<T>>,
		<T as pezframe_system::Config>::RuntimeEvent: TryInto<pezpallet::Event<T>>,
		pezsp_statement_store::BlockHash: From<<T as pezframe_system::Config>::Hash>,
	{
		fn offchain_worker(now: BlockNumberFor<T>) {
			log::trace!(target: LOG_TARGET, "Collecting statements at #{:?}", now);
			Pezpallet::<T>::collect_statements();
		}
	}
}

impl<T: Config> Pezpallet<T>
where
	<T as pezframe_system::Config>::AccountId: From<pezsp_statement_store::AccountId>,
	pezsp_statement_store::AccountId: From<<T as pezframe_system::Config>::AccountId>,
	<T as pezframe_system::Config>::RuntimeEvent: From<pezpallet::Event<T>>,
	<T as pezframe_system::Config>::RuntimeEvent: TryInto<pezpallet::Event<T>>,
	pezsp_statement_store::BlockHash: From<<T as pezframe_system::Config>::Hash>,
{
	/// Validate a statement against current state. This is supposed to be called by the statement
	/// store on the host side.
	pub fn validate_statement(
		_source: StatementSource,
		mut statement: Statement,
	) -> Result<ValidStatement, InvalidStatement> {
		pezsp_io::init_tracing();
		log::debug!(target: LOG_TARGET, "Validating statement {:?}", statement);
		let account: T::AccountId = match statement.proof() {
			Some(Proof::OnChain { who, block_hash, event_index }) => {
				if pezframe_system::Pezpallet::<T>::parent_hash().as_ref() != block_hash.as_slice()
				{
					log::debug!(target: LOG_TARGET, "Bad block hash.");
					return Err(InvalidStatement::BadProof);
				}
				let account: T::AccountId = (*who).into();
				match pezframe_system::Pezpallet::<T>::event_no_consensus(*event_index as usize) {
					Some(e) => {
						statement.remove_proof();
						if let Ok(Event::NewStatement { account: a, statement: s }) = e.try_into() {
							if a != account || s != statement {
								log::debug!(target: LOG_TARGET, "Event data mismatch");
								return Err(InvalidStatement::BadProof);
							}
						} else {
							log::debug!(target: LOG_TARGET, "Event type mismatch");
							return Err(InvalidStatement::BadProof);
						}
					},
					_ => {
						log::debug!(target: LOG_TARGET, "Bad event index");
						return Err(InvalidStatement::BadProof);
					},
				}
				account
			},
			_ => match statement.verify_signature() {
				SignatureVerificationResult::Valid(account) => account.into(),
				SignatureVerificationResult::Invalid => {
					log::debug!(target: LOG_TARGET, "Bad statement signature.");
					return Err(InvalidStatement::BadProof);
				},
				SignatureVerificationResult::NoSignature => {
					log::debug!(target: LOG_TARGET, "Missing statement signature.");
					return Err(InvalidStatement::NoProof);
				},
			},
		};
		let statement_cost = T::StatementCost::get();
		let byte_cost = T::ByteCost::get();
		let balance = <T::Currency as Inspect<AccountIdOf<T>>>::balance(&account);
		let min_allowed_statements = T::MinAllowedStatements::get();
		let max_allowed_statements = T::MaxAllowedStatements::get();
		let min_allowed_bytes = T::MinAllowedBytes::get();
		let max_allowed_bytes = T::MaxAllowedBytes::get();
		let max_count = balance
			.checked_div(&statement_cost)
			.unwrap_or_default()
			.saturated_into::<u32>()
			.clamp(min_allowed_statements, max_allowed_statements);
		let max_size = balance
			.checked_div(&byte_cost)
			.unwrap_or_default()
			.saturated_into::<u32>()
			.clamp(min_allowed_bytes, max_allowed_bytes);

		Ok(ValidStatement { max_count, max_size })
	}

	/// Submit a statement event. The statement will be picked up by the offchain worker and
	/// broadcast to the network.
	pub fn submit_statement(account: T::AccountId, statement: Statement) {
		Self::deposit_event(Event::NewStatement { account, statement });
	}

	fn collect_statements() {
		// Find `NewStatement` events and submit them to the store
		for (index, event) in
			pezframe_system::Pezpallet::<T>::read_events_no_consensus().enumerate()
		{
			if let Ok(Event::<T>::NewStatement { account, mut statement }) = event.event.try_into()
			{
				if statement.proof().is_none() {
					let proof = Proof::OnChain {
						who: account.into(),
						block_hash: pezframe_system::Pezpallet::<T>::parent_hash().into(),
						event_index: index as u64,
					};
					statement.set_proof(proof);
				}
				pezsp_statement_store::runtime_api::statement_store::submit_statement(statement);
			}
		}
	}
}
