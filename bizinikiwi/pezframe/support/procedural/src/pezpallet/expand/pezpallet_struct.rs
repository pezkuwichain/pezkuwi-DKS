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

use crate::pezpallet::{expand::merge_where_clauses, Def};
use pezframe_support_procedural_tools::get_doc_literals;

///
/// * Add derive trait on Pezpallet
/// * Implement GetStorageVersion on Pezpallet
/// * Implement OnGenesis on Pezpallet
/// * Implement `fn error_metadata` on Pezpallet
/// * declare Module type alias for construct_runtime
/// * replace the first field type of `struct Pezpallet` with `PhantomData` if it is `_`
/// * implementation of `PalletInfoAccess` information
/// * implementation of `StorageInfoTrait` on Pezpallet
pub fn expand_pallet_struct(def: &mut Def) -> proc_macro2::TokenStream {
	let pezframe_support = &def.pezframe_support;
	let pezframe_system = &def.pezframe_system;
	let type_impl_gen = &def.type_impl_generics(def.pezpallet_struct.attr_span);
	let type_use_gen = &def.type_use_generics(def.pezpallet_struct.attr_span);
	let type_decl_gen = &def.type_decl_generics(def.pezpallet_struct.attr_span);
	let pezpallet_ident = &def.pezpallet_struct.pezpallet;
	let config_where_clause = &def.config.where_clause;
	let deprecation_status = match crate::deprecation::get_deprecation(
		&quote::quote! {#pezframe_support},
		&def.item.attrs,
	) {
		Ok(deprecation) => deprecation,
		Err(e) => return e.into_compile_error(),
	};

	let mut storages_where_clauses = vec![&def.config.where_clause];
	storages_where_clauses.extend(def.storages.iter().map(|storage| &storage.where_clause));
	let storages_where_clauses = merge_where_clauses(&storages_where_clauses);

	let pezpallet_item = {
		let pezpallet_module_items = &mut def.item.content.as_mut().expect("Checked by def").1;
		let item = &mut pezpallet_module_items[def.pezpallet_struct.index];
		if let syn::Item::Struct(item) = item {
			item
		} else {
			unreachable!("Checked by pezpallet struct parser")
		}
	};

	// If the first field type is `_` then we replace with `PhantomData`
	if let Some(field) = pezpallet_item.fields.iter_mut().next() {
		if field.ty == syn::parse_quote!(_) {
			field.ty = syn::parse_quote!(
				core::marker::PhantomData<(#type_use_gen)>
			);
		}
	}

	if get_doc_literals(&pezpallet_item.attrs).is_empty() {
		pezpallet_item.attrs.push(syn::parse_quote!(
			#[doc = r"
				The `Pezpallet` struct, the main type that implements traits and standalone
				functions within the pezpallet.
			"]
		));
	}

	pezpallet_item.attrs.push(syn::parse_quote!(
		#[derive(
			#pezframe_support::CloneNoBound,
			#pezframe_support::EqNoBound,
			#pezframe_support::PartialEqNoBound,
			#pezframe_support::RuntimeDebugNoBound,
		)]
	));

	let pezpallet_error_metadata = if let Some(error_def) = &def.error {
		let error_ident = &error_def.error;
		quote::quote_spanned!(def.pezpallet_struct.attr_span =>
			impl<#type_impl_gen> #pezpallet_ident<#type_use_gen> #config_where_clause {
				#[doc(hidden)]
				#[allow(deprecated)]
				pub fn error_metadata() -> Option<#pezframe_support::__private::metadata_ir::PalletErrorMetadataIR> {
					Some(<#error_ident<#type_use_gen>>::error_metadata())
				}
			}
		)
	} else {
		quote::quote_spanned!(def.pezpallet_struct.attr_span =>
			impl<#type_impl_gen> #pezpallet_ident<#type_use_gen> #config_where_clause {
				#[doc(hidden)]
				pub fn error_metadata() -> Option<#pezframe_support::__private::metadata_ir::PalletErrorMetadataIR> {
					None
				}
			}
		)
	};

	let storage_info_span = def
		.pezpallet_struct
		.without_storage_info
		.unwrap_or(def.pezpallet_struct.attr_span);

	let storage_names = &def.storages.iter().map(|storage| &storage.ident).collect::<Vec<_>>();
	let storage_cfg_attrs =
		&def.storages.iter().map(|storage| &storage.cfg_attrs).collect::<Vec<_>>();
	let storage_maybe_allow_attrs = &def
		.storages
		.iter()
		.map(|storage| crate::deprecation::extract_or_return_allow_attrs(&storage.attrs).collect())
		.collect::<Vec<Vec<_>>>();
	// Depending on the flag `without_storage_info` and the storage attribute `unbounded`, we use
	// partial or full storage info from storage.
	let storage_info_traits = &def
		.storages
		.iter()
		.map(|storage| {
			if storage.unbounded || def.pezpallet_struct.without_storage_info.is_some() {
				quote::quote_spanned!(storage_info_span => PartialStorageInfoTrait)
			} else {
				quote::quote_spanned!(storage_info_span => StorageInfoTrait)
			}
		})
		.collect::<Vec<_>>();

	let storage_info_methods = &def
		.storages
		.iter()
		.map(|storage| {
			if storage.unbounded || def.pezpallet_struct.without_storage_info.is_some() {
				quote::quote_spanned!(storage_info_span => partial_storage_info)
			} else {
				quote::quote_spanned!(storage_info_span => storage_info)
			}
		})
		.collect::<Vec<_>>();

	let storage_info = quote::quote_spanned!(storage_info_span =>
		impl<#type_impl_gen> #pezframe_support::traits::StorageInfoTrait
			for #pezpallet_ident<#type_use_gen>
			#storages_where_clauses
		{
			fn storage_info()
				-> #pezframe_support::__private::Vec<#pezframe_support::traits::StorageInfo>
			{
				#[allow(unused_mut)]
				let mut res = #pezframe_support::__private::vec![];

				#(
					#(#storage_cfg_attrs)*
					#(#storage_maybe_allow_attrs)*
					{
						let mut storage_info = <
							#storage_names<#type_use_gen>
							as #pezframe_support::traits::#storage_info_traits
						>::#storage_info_methods();
						res.append(&mut storage_info);
					}
				)*

				res
			}
		}
	);

	let (storage_version, in_code_storage_version_ty) =
		if let Some(v) = def.pezpallet_struct.storage_version.as_ref() {
			(quote::quote! { #v }, quote::quote! { #pezframe_support::traits::StorageVersion })
		} else {
			(
				quote::quote! { core::default::Default::default() },
				quote::quote! { #pezframe_support::traits::NoStorageVersionSet },
			)
		};

	let whitelisted_storage_idents: Vec<syn::Ident> = def
		.storages
		.iter()
		.filter_map(|s| s.whitelisted.then(|| s.ident.clone()))
		.collect();

	let whitelisted_storage_keys_impl = quote::quote![
		use #pezframe_support::traits::{StorageInfoTrait, TrackedStorageKey, WhitelistedStorageKeys};
		impl<#type_impl_gen> WhitelistedStorageKeys for #pezpallet_ident<#type_use_gen> #storages_where_clauses {
			fn whitelisted_storage_keys() -> #pezframe_support::__private::Vec<TrackedStorageKey> {
				use #pezframe_support::__private::vec;
				vec![#(
					TrackedStorageKey::new(#whitelisted_storage_idents::<#type_use_gen>::hashed_key().to_vec())
				),*]
			}
		}
	];

	quote::quote_spanned!(def.pezpallet_struct.attr_span =>
		#pezpallet_error_metadata

		/// Type alias to `Pezpallet`, to be used by `construct_runtime`.
		///
		/// Generated by `pezpallet` attribute macro.
		#[deprecated(note = "use `Pezpallet` instead")]
		#[allow(dead_code)]
		pub type Module<#type_decl_gen> = #pezpallet_ident<#type_use_gen>;

		// Implement `GetStorageVersion` for `Pezpallet`
		impl<#type_impl_gen> #pezframe_support::traits::GetStorageVersion
			for #pezpallet_ident<#type_use_gen>
			#config_where_clause
		{
			type InCodeStorageVersion = #in_code_storage_version_ty;

			fn in_code_storage_version() -> Self::InCodeStorageVersion {
				#storage_version
			}

			fn on_chain_storage_version() -> #pezframe_support::traits::StorageVersion {
				#pezframe_support::traits::StorageVersion::get::<Self>()
			}
		}

		// Implement `OnGenesis` for `Pezpallet`
		impl<#type_impl_gen> #pezframe_support::traits::OnGenesis
			for #pezpallet_ident<#type_use_gen>
			#config_where_clause
		{
			fn on_genesis() {
				let storage_version: #pezframe_support::traits::StorageVersion = #storage_version;
				storage_version.put::<Self>();
			}
		}

		// Implement `PalletInfoAccess` for `Pezpallet`
		impl<#type_impl_gen> #pezframe_support::traits::PalletInfoAccess
			for #pezpallet_ident<#type_use_gen>
			#config_where_clause
		{
			fn index() -> usize {
				<
					<T as #pezframe_system::Config>::PalletInfo as #pezframe_support::traits::PalletInfo
				>::index::<Self>()
					.expect("Pezpallet is part of the runtime because pezpallet `Config` trait is \
						implemented by the runtime")
			}

			fn name() -> &'static str {
				<
					<T as #pezframe_system::Config>::PalletInfo as #pezframe_support::traits::PalletInfo
				>::name::<Self>()
					.expect("Pezpallet is part of the runtime because pezpallet `Config` trait is \
						implemented by the runtime")
			}

			fn name_hash() -> [u8; 16] {
				<
					<T as #pezframe_system::Config>::PalletInfo as #pezframe_support::traits::PalletInfo
				>::name_hash::<Self>()
					.expect("Pezpallet is part of the runtime because pezpallet `Config` trait is \
						implemented by the runtime")
			}

			fn module_name() -> &'static str {
				<
					<T as #pezframe_system::Config>::PalletInfo as #pezframe_support::traits::PalletInfo
				>::module_name::<Self>()
					.expect("Pezpallet is part of the runtime because pezpallet `Config` trait is \
						implemented by the runtime")
			}

			fn crate_version() -> #pezframe_support::traits::CrateVersion {
				#pezframe_support::crate_to_crate_version!()
			}
		}

		impl<#type_impl_gen> #pezframe_support::traits::PalletsInfoAccess
			for #pezpallet_ident<#type_use_gen>
			#config_where_clause
		{
			fn count() -> usize { 1 }
			fn infos() -> #pezframe_support::__private::Vec<#pezframe_support::traits::PalletInfoData> {
				use #pezframe_support::traits::PalletInfoAccess;
				let item = #pezframe_support::traits::PalletInfoData {
					index: Self::index(),
					name: Self::name(),
					module_name: Self::module_name(),
					crate_version: Self::crate_version(),
				};
				#pezframe_support::__private::vec![item]
			}
		}

		#storage_info
		#whitelisted_storage_keys_impl

		impl<#type_use_gen> #pezpallet_ident<#type_use_gen> {
			#[allow(dead_code)]
			#[doc(hidden)]
			pub fn deprecation_info() -> #pezframe_support::__private::metadata_ir::ItemDeprecationInfoIR {
				#deprecation_status
			}
		}
	)
}
