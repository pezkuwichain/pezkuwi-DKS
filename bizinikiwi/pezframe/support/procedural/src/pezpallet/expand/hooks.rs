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

use crate::pezpallet::Def;

/// * implement the individual traits using the Hooks trait
pub fn expand_hooks(def: &mut Def) -> proc_macro2::TokenStream {
	let (where_clause, span, has_runtime_upgrade) = match def.hooks.as_ref() {
		Some(hooks) => {
			let where_clause = hooks.where_clause.clone();
			let span = hooks.attr_span;
			let has_runtime_upgrade = hooks.has_runtime_upgrade;
			(where_clause, span, has_runtime_upgrade)
		},
		None => (def.config.where_clause.clone(), def.pezpallet_struct.attr_span, false),
	};

	let pezframe_support = &def.pezframe_support;
	let type_impl_gen = &def.type_impl_generics(span);
	let type_use_gen = &def.type_use_generics(span);
	let pezpallet_ident = &def.pezpallet_struct.pezpallet;
	let pezframe_system = &def.pezframe_system;
	let pezpallet_name = quote::quote! {
		<
			<T as #pezframe_system::Config>::PalletInfo
			as
			#pezframe_support::traits::PalletInfo
		>::name::<Self>().unwrap_or("<unknown pezpallet name>")
	};

	let initialize_on_chain_storage_version = if let Some(in_code_version) =
		&def.pezpallet_struct.storage_version
	{
		quote::quote! {
			#pezframe_support::__private::log::info!(
				target: #pezframe_support::LOG_TARGET,
				"🐥 New pezpallet {:?} detected in the runtime. Initializing the on-chain storage version to match the storage version defined in the pezpallet: {:?}",
				#pezpallet_name,
				#in_code_version
			);
			#in_code_version.put::<Self>();
		}
	} else {
		quote::quote! {
			let default_version = #pezframe_support::traits::StorageVersion::new(0);
			#pezframe_support::__private::log::info!(
				target: #pezframe_support::LOG_TARGET,
				"🐥 New pezpallet {:?} detected in the runtime. The pezpallet has no defined storage version, so the on-chain version is being initialized to {:?}.",
				#pezpallet_name,
				default_version
			);
			default_version.put::<Self>();
		}
	};

	let log_runtime_upgrade = if has_runtime_upgrade {
		// a migration is defined here.
		quote::quote! {
			#pezframe_support::__private::log::info!(
				target: #pezframe_support::LOG_TARGET,
				"⚠️ {} declares internal migrations (which *might* execute). \
				 On-chain `{:?}` vs in-code storage version `{:?}`",
				#pezpallet_name,
				<Self as #pezframe_support::traits::GetStorageVersion>::on_chain_storage_version(),
				<Self as #pezframe_support::traits::GetStorageVersion>::in_code_storage_version(),
			);
		}
	} else {
		// default.
		quote::quote! {
			#pezframe_support::__private::log::debug!(
				target: #pezframe_support::LOG_TARGET,
				"✅ no migration for {}",
				#pezpallet_name,
			);
		}
	};

	let hooks_impl = if def.hooks.is_none() {
		let pezframe_system = &def.pezframe_system;
		quote::quote! {
			impl<#type_impl_gen>
				#pezframe_support::traits::Hooks<#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>>
				for #pezpallet_ident<#type_use_gen> #where_clause {}
		}
	} else {
		proc_macro2::TokenStream::new()
	};

	// If a storage version is set, we should ensure that the storage version on chain matches the
	// in-code storage version. This assumes that `Executive` is running custom migrations before
	// the pallets are called.
	let post_storage_version_check = if def.pezpallet_struct.storage_version.is_some() {
		quote::quote! {
			let on_chain_version = <Self as #pezframe_support::traits::GetStorageVersion>::on_chain_storage_version();
			let in_code_version = <Self as #pezframe_support::traits::GetStorageVersion>::in_code_storage_version();

			if on_chain_version != in_code_version {
				#pezframe_support::__private::log::error!(
					target: #pezframe_support::LOG_TARGET,
					"{}: On chain storage version {:?} doesn't match in-code storage version {:?}.",
					#pezpallet_name,
					on_chain_version,
					in_code_version,
				);

				return Err("On chain and in-code storage version do not match. Missing runtime upgrade?".into());
			}
		}
	} else {
		quote::quote! {
			let on_chain_version = <Self as #pezframe_support::traits::GetStorageVersion>::on_chain_storage_version();

			if on_chain_version != #pezframe_support::traits::StorageVersion::new(0) {
				#pezframe_support::__private::log::error!(
					target: #pezframe_support::LOG_TARGET,
					"{}: On chain storage version {:?} is set to non zero, \
					 while the pezpallet is missing the `#[pezpallet::storage_version(VERSION)]` attribute.",
					#pezpallet_name,
					on_chain_version,
				);

				return Err("On chain storage version set, while the pezpallet doesn't \
							have the `#[pezpallet::storage_version(VERSION)]` attribute.".into());
			}
		}
	};

	quote::quote_spanned!(span =>
		#hooks_impl

		impl<#type_impl_gen>
			#pezframe_support::traits::OnFinalize<#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>>
			for #pezpallet_ident<#type_use_gen> #where_clause
		{
			fn on_finalize(n: #pezframe_system::pezpallet_prelude::BlockNumberFor::<T>) {
				#pezframe_support::__private::pezsp_tracing::enter_span!(
					#pezframe_support::__private::pezsp_tracing::trace_span!("on_finalize")
				);
				<
					Self as #pezframe_support::traits::Hooks<
						#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>
					>
				>::on_finalize(n)
			}
		}

		impl<#type_impl_gen>
			#pezframe_support::traits::OnIdle<#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>>
			for #pezpallet_ident<#type_use_gen> #where_clause
		{
			fn on_idle(
				n: #pezframe_system::pezpallet_prelude::BlockNumberFor::<T>,
				remaining_weight: #pezframe_support::weights::Weight
			) -> #pezframe_support::weights::Weight {
				<
					Self as #pezframe_support::traits::Hooks<
						#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>
					>
				>::on_idle(n, remaining_weight)
			}
		}

		impl<#type_impl_gen>
			#pezframe_support::traits::OnPoll<#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>>
			for #pezpallet_ident<#type_use_gen> #where_clause
		{
			fn on_poll(
				n: #pezframe_system::pezpallet_prelude::BlockNumberFor::<T>,
				weight: &mut #pezframe_support::weights::WeightMeter
			) {
				<
					Self as #pezframe_support::traits::Hooks<
						#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>
					>
				>::on_poll(n, weight);
			}
		}

		impl<#type_impl_gen>
			#pezframe_support::traits::OnInitialize<#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>>
			for #pezpallet_ident<#type_use_gen> #where_clause
		{
			fn on_initialize(
				n: #pezframe_system::pezpallet_prelude::BlockNumberFor::<T>
			) -> #pezframe_support::weights::Weight {
				#pezframe_support::__private::pezsp_tracing::enter_span!(
					#pezframe_support::__private::pezsp_tracing::trace_span!("on_initialize")
				);
				<
					Self as #pezframe_support::traits::Hooks<
						#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>
					>
				>::on_initialize(n)
			}
		}

		impl<#type_impl_gen>
			#pezframe_support::traits::BeforeAllRuntimeMigrations
			for #pezpallet_ident<#type_use_gen> #where_clause
		{
			fn before_all_runtime_migrations() -> #pezframe_support::weights::Weight {
				use #pezframe_support::traits::{Get, PalletInfoAccess};
				use #pezframe_support::__private::hashing::twox_128;
				use #pezframe_support::storage::unhashed::contains_prefixed_key;
				#pezframe_support::__private::pezsp_tracing::enter_span!(
					#pezframe_support::__private::pezsp_tracing::trace_span!("before_all")
				);

				// Check if the pezpallet has any keys set, including the storage version. If there are
				// no keys set, the pezpallet was just added to the runtime and needs to have its
				// version initialized.
				let pezpallet_hashed_prefix = <Self as PalletInfoAccess>::name_hash();
				let exists = contains_prefixed_key(&pezpallet_hashed_prefix);
				if !exists {
					#initialize_on_chain_storage_version
					<T as #pezframe_system::Config>::DbWeight::get().reads_writes(1, 1)
				} else {
					<T as #pezframe_system::Config>::DbWeight::get().reads(1)
				}
			}
		}

		impl<#type_impl_gen>
			#pezframe_support::traits::OnRuntimeUpgrade
			for #pezpallet_ident<#type_use_gen> #where_clause
		{
			fn on_runtime_upgrade() -> #pezframe_support::weights::Weight {
				#pezframe_support::__private::pezsp_tracing::enter_span!(
					#pezframe_support::__private::pezsp_tracing::trace_span!("on_runtime_update")
				);

				// log info about the upgrade.
				#log_runtime_upgrade

				<
					Self as #pezframe_support::traits::Hooks<
						#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>
					>
				>::on_runtime_upgrade()
			}

			#pezframe_support::try_runtime_enabled! {
				fn pre_upgrade() -> Result<#pezframe_support::__private::Vec<u8>, #pezframe_support::pezsp_runtime::TryRuntimeError> {
					<
						Self
						as
						#pezframe_support::traits::Hooks<#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>>
					>::pre_upgrade()
				}

				fn post_upgrade(state: #pezframe_support::__private::Vec<u8>) -> Result<(), #pezframe_support::pezsp_runtime::TryRuntimeError> {
					#post_storage_version_check

					<
						Self
						as
						#pezframe_support::traits::Hooks<#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>>
					>::post_upgrade(state)
				}
			}
		}

		impl<#type_impl_gen>
			#pezframe_support::traits::OffchainWorker<#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>>
			for #pezpallet_ident<#type_use_gen> #where_clause
		{
			fn offchain_worker(n: #pezframe_system::pezpallet_prelude::BlockNumberFor::<T>) {
				<
					Self as #pezframe_support::traits::Hooks<
						#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>
					>
				>::offchain_worker(n)
			}
		}

		// Integrity tests are only required for when `std` is enabled.
		#pezframe_support::std_enabled! {
			impl<#type_impl_gen>
				#pezframe_support::traits::IntegrityTest
			for #pezpallet_ident<#type_use_gen> #where_clause
			{
				fn integrity_test() {
					#pezframe_support::__private::pezsp_io::TestExternalities::default().execute_with(|| {
						<
							Self as #pezframe_support::traits::Hooks<
								#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>
							>
						>::integrity_test()
					});
				}
			}
		}

		#pezframe_support::try_runtime_enabled! {
			impl<#type_impl_gen>
				#pezframe_support::traits::TryState<#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>>
				for #pezpallet_ident<#type_use_gen> #where_clause
			{
				fn try_state(
					n: #pezframe_system::pezpallet_prelude::BlockNumberFor::<T>,
					_s: #pezframe_support::traits::TryStateSelect
				) -> Result<(), #pezframe_support::pezsp_runtime::TryRuntimeError> {
					#pezframe_support::__private::log::info!(
						target: #pezframe_support::LOG_TARGET,
						"🩺 Running {:?} try-state checks",
						#pezpallet_name,
					);
					<
						Self as #pezframe_support::traits::Hooks<
							#pezframe_system::pezpallet_prelude::BlockNumberFor::<T>
						>
					>::try_state(n).inspect_err(|err| {
						#pezframe_support::__private::log::error!(
							target: #pezframe_support::LOG_TARGET,
							"❌ {:?} try_state checks failed: {:?}",
							#pezpallet_name,
							err
						);
					})
				}
			}
		}
	)
}
