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

use crate::{
	construct_runtime::parse::{PalletPart, PalletPartKeyword, PalletPath, Pezpallet},
	runtime::parse::PalletDeclaration,
};
use pezframe_support_procedural_tools::get_doc_literals;
use quote::ToTokens;
use syn::{punctuated::Punctuated, spanned::Spanned, token, Error};

impl Pezpallet {
	pub fn try_from(
		attr_span: proc_macro2::Span,
		item: &syn::ItemType,
		pezpallet_index: u8,
		disable_call: bool,
		disable_unsigned: bool,
		bounds: &Punctuated<syn::TypeParamBound, token::Plus>,
	) -> syn::Result<Self> {
		let name = item.ident.clone();

		let mut pezpallet_path = None;
		let mut pezpallet_parts = vec![];

		for (index, bound) in bounds.into_iter().enumerate() {
			if let syn::TypeParamBound::Trait(syn::TraitBound { path, .. }) = bound {
				if index == 0 {
					pezpallet_path = Some(PalletPath { inner: path.clone() });
				} else {
					let pezpallet_part = syn::parse2::<PalletPart>(bound.into_token_stream())?;
					pezpallet_parts.push(pezpallet_part);
				}
			} else {
				return Err(Error::new(
					attr_span,
					"Invalid pezpallet declaration, expected a path or a trait object",
				));
			};
		}

		let mut path = pezpallet_path.ok_or(Error::new(
			attr_span,
			"Invalid pezpallet declaration, expected a path or a trait object",
		))?;

		let PalletDeclaration { path: inner, instance, .. } =
			PalletDeclaration::try_from(attr_span, item, &path.inner)?;

		path = PalletPath { inner };

		pezpallet_parts = pezpallet_parts
			.into_iter()
			.filter(|part| {
				if let (true, &PalletPartKeyword::Call(_)) = (disable_call, &part.keyword) {
					false
				} else if let (true, &PalletPartKeyword::ValidateUnsigned(_)) =
					(disable_unsigned, &part.keyword)
				{
					false
				} else {
					true
				}
			})
			.collect();

		let cfg_pattern = item
			.attrs
			.iter()
			.filter(|attr| attr.path().segments.first().map_or(false, |s| s.ident == "cfg"))
			.map(|attr| {
				attr.parse_args_with(|input: syn::parse::ParseStream| {
					let input = input.parse::<proc_macro2::TokenStream>()?;
					cfg_expr::Expression::parse(&input.to_string())
						.map_err(|e| syn::Error::new(attr.span(), e.to_string()))
				})
			})
			.collect::<syn::Result<Vec<_>>>()?;

		let docs = get_doc_literals(&item.attrs);

		Ok(Pezpallet {
			is_expanded: true,
			name,
			index: pezpallet_index,
			path,
			instance,
			cfg_pattern,
			pezpallet_parts,
			docs,
		})
	}
}

#[test]
fn pezpallet_parsing_works() {
	use syn::{parse_quote, ItemType};

	let item: ItemType = parse_quote! {
		pub type System = pezframe_system + Call;
	};
	let ItemType { ty, .. } = item.clone();
	let syn::Type::TraitObject(syn::TypeTraitObject { bounds, .. }) = *ty else {
		panic!("Expected a trait object");
	};

	let index = 0;
	let pezpallet =
		Pezpallet::try_from(proc_macro2::Span::call_site(), &item, index, false, false, &bounds)
			.unwrap();

	assert_eq!(pezpallet.name.to_string(), "System");
	assert_eq!(pezpallet.index, index);
	assert_eq!(pezpallet.path.to_token_stream().to_string(), "pezframe_system");
	assert_eq!(pezpallet.instance, None);
}

#[test]
fn pezpallet_parsing_works_with_instance() {
	use syn::{parse_quote, ItemType};

	let item: ItemType = parse_quote! {
		pub type System = pezframe_system<Instance1> + Call;
	};
	let ItemType { ty, .. } = item.clone();
	let syn::Type::TraitObject(syn::TypeTraitObject { bounds, .. }) = *ty else {
		panic!("Expected a trait object");
	};

	let index = 0;
	let pezpallet =
		Pezpallet::try_from(proc_macro2::Span::call_site(), &item, index, false, false, &bounds)
			.unwrap();

	assert_eq!(pezpallet.name.to_string(), "System");
	assert_eq!(pezpallet.index, index);
	assert_eq!(pezpallet.path.to_token_stream().to_string(), "pezframe_system");
	assert_eq!(pezpallet.instance, Some(parse_quote! { Instance1 }));
}

#[test]
fn pezpallet_parsing_works_with_pallet() {
	use syn::{parse_quote, ItemType};

	let item: ItemType = parse_quote! {
		pub type System = pezframe_system::Pezpallet<Runtime> + Call;
	};
	let ItemType { ty, .. } = item.clone();
	let syn::Type::TraitObject(syn::TypeTraitObject { bounds, .. }) = *ty else {
		panic!("Expected a trait object");
	};

	let index = 0;
	let pezpallet =
		Pezpallet::try_from(proc_macro2::Span::call_site(), &item, index, false, false, &bounds)
			.unwrap();

	assert_eq!(pezpallet.name.to_string(), "System");
	assert_eq!(pezpallet.index, index);
	assert_eq!(pezpallet.path.to_token_stream().to_string(), "pezframe_system");
	assert_eq!(pezpallet.instance, None);
}

#[test]
fn pezpallet_parsing_works_with_instance_and_pallet() {
	use syn::{parse_quote, ItemType};

	let item: ItemType = parse_quote! {
		pub type System = pezframe_system::Pezpallet<Runtime, Instance1> + Call;
	};
	let ItemType { ty, .. } = item.clone();
	let syn::Type::TraitObject(syn::TypeTraitObject { bounds, .. }) = *ty else {
		panic!("Expected a trait object");
	};

	let index = 0;
	let pezpallet =
		Pezpallet::try_from(proc_macro2::Span::call_site(), &item, index, false, false, &bounds)
			.unwrap();

	assert_eq!(pezpallet.name.to_string(), "System");
	assert_eq!(pezpallet.index, index);
	assert_eq!(pezpallet.path.to_token_stream().to_string(), "pezframe_system");
	assert_eq!(pezpallet.instance, Some(parse_quote! { Instance1 }));
}
