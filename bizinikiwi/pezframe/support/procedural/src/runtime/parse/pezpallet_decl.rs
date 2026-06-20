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

use syn::{Ident, PathArguments};

/// The declaration of a pezpallet.
#[derive(Debug, Clone)]
pub struct PalletDeclaration {
	/// The name of the pezpallet, e.g.`System` in `pub type System = pezframe_system`.
	pub name: Ident,
	/// The path of the pezpallet, e.g. `pezframe_system` in `pub type System = pezframe_system`.
	pub path: syn::Path,
	/// The segment of the pezpallet, e.g. `Pezpallet` in `pub type System =
	/// pezframe_system::Pezpallet`.
	pub pezpallet_segment: Option<syn::PathSegment>,
	/// The runtime parameter of the pezpallet, e.g. `Runtime` in
	/// `pub type System = pezframe_system::Pezpallet<Runtime>`.
	pub runtime_param: Option<Ident>,
	/// The instance of the pezpallet, e.g. `Instance1` in `pub type Council =
	/// pezpallet_collective<Instance1>`.
	pub instance: Option<Ident>,
}

impl PalletDeclaration {
	pub fn try_from(
		_attr_span: proc_macro2::Span,
		item: &syn::ItemType,
		path: &syn::Path,
	) -> syn::Result<Self> {
		let name = item.ident.clone();

		let mut path = path.clone();

		let mut pezpallet_segment = None;
		let mut runtime_param = None;
		let mut instance = None;
		if let Some(segment) = path.segments.iter_mut().find(|seg| !seg.arguments.is_empty()) {
			if let PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
				args, ..
			}) = segment.arguments.clone()
			{
				if segment.ident == "Pezpallet" {
					let mut segment = segment.clone();
					segment.arguments = PathArguments::None;
					pezpallet_segment = Some(segment.clone());
				}
				let mut args_iter = args.iter();
				if let Some(syn::GenericArgument::Type(syn::Type::Path(arg_path))) =
					args_iter.next()
				{
					let ident = arg_path.path.require_ident()?.clone();
					if segment.ident == "Pezpallet" {
						runtime_param = Some(ident);
						if let Some(syn::GenericArgument::Type(syn::Type::Path(arg_path))) =
							args_iter.next()
						{
							instance = Some(arg_path.path.require_ident()?.clone());
						}
					} else {
						instance = Some(ident);
						segment.arguments = PathArguments::None;
					}
				}
			}
		}

		if pezpallet_segment.is_some() {
			path = syn::Path {
				leading_colon: None,
				segments: path
					.segments
					.iter()
					.filter(|seg| seg.arguments.is_empty())
					.cloned()
					.collect(),
			};
		}

		Ok(Self { name, path, pezpallet_segment, runtime_param, instance })
	}
}

#[test]
fn declaration_works() {
	use syn::parse_quote;

	let decl: PalletDeclaration = PalletDeclaration::try_from(
		proc_macro2::Span::call_site(),
		&parse_quote! { pub type System = pezframe_system; },
		&parse_quote! { pezframe_system },
	)
	.expect("Failed to parse pezpallet declaration");

	assert_eq!(decl.name, "System");
	assert_eq!(decl.path, parse_quote! { pezframe_system });
	assert_eq!(decl.pezpallet_segment, None);
	assert_eq!(decl.runtime_param, None);
	assert_eq!(decl.instance, None);
}

#[test]
fn declaration_works_with_instance() {
	use syn::parse_quote;

	let decl: PalletDeclaration = PalletDeclaration::try_from(
		proc_macro2::Span::call_site(),
		&parse_quote! { pub type System = pezframe_system<Instance1>; },
		&parse_quote! { pezframe_system<Instance1> },
	)
	.expect("Failed to parse pezpallet declaration");

	assert_eq!(decl.name, "System");
	assert_eq!(decl.path, parse_quote! { pezframe_system });
	assert_eq!(decl.pezpallet_segment, None);
	assert_eq!(decl.runtime_param, None);
	assert_eq!(decl.instance, Some(parse_quote! { Instance1 }));
}

#[test]
fn declaration_works_with_pallet() {
	use syn::parse_quote;

	let decl: PalletDeclaration = PalletDeclaration::try_from(
		proc_macro2::Span::call_site(),
		&parse_quote! { pub type System = pezframe_system::Pezpallet<Runtime>; },
		&parse_quote! { pezframe_system::Pezpallet<Runtime> },
	)
	.expect("Failed to parse pezpallet declaration");

	assert_eq!(decl.name, "System");
	assert_eq!(decl.path, parse_quote! { pezframe_system });

	let segment: syn::PathSegment =
		syn::PathSegment { ident: parse_quote! { Pezpallet }, arguments: PathArguments::None };
	assert_eq!(decl.pezpallet_segment, Some(segment));
	assert_eq!(decl.runtime_param, Some(parse_quote! { Runtime }));
	assert_eq!(decl.instance, None);
}

#[test]
fn declaration_works_with_pallet_and_instance() {
	use syn::parse_quote;

	let decl: PalletDeclaration = PalletDeclaration::try_from(
		proc_macro2::Span::call_site(),
		&parse_quote! { pub type System = pezframe_system::Pezpallet<Runtime, Instance1>; },
		&parse_quote! { pezframe_system::Pezpallet<Runtime, Instance1> },
	)
	.expect("Failed to parse pezpallet declaration");

	assert_eq!(decl.name, "System");
	assert_eq!(decl.path, parse_quote! { pezframe_system });

	let segment: syn::PathSegment =
		syn::PathSegment { ident: parse_quote! { Pezpallet }, arguments: PathArguments::None };
	assert_eq!(decl.pezpallet_segment, Some(segment));
	assert_eq!(decl.runtime_param, Some(parse_quote! { Runtime }));
	assert_eq!(decl.instance, Some(parse_quote! { Instance1 }));
}
