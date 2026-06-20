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

use crate::pezpallet::{parse::view_functions::ViewFunctionDef, Def};
use proc_macro2::{Span, TokenStream};
use syn::spanned::Spanned;

pub fn expand_view_functions(def: &Def) -> TokenStream {
	let (span, where_clause, view_fns) = match def.view_functions.as_ref() {
		Some(view_fns) => {
			(view_fns.attr_span, view_fns.where_clause.clone(), view_fns.view_functions.clone())
		},
		None => (def.item.span(), def.config.where_clause.clone(), Vec::new()),
	};

	let view_function_prefix_impl =
		expand_view_function_prefix_impl(def, span, where_clause.as_ref());

	let view_fn_impls = view_fns
		.iter()
		.map(|view_fn| expand_view_function(def, span, where_clause.as_ref(), view_fn));
	let impl_dispatch_view_function =
		impl_dispatch_view_function(def, span, where_clause.as_ref(), &view_fns);
	let impl_view_function_metadata =
		impl_view_function_metadata(def, span, where_clause.as_ref(), &view_fns);

	quote::quote! {
		#view_function_prefix_impl
		#( #view_fn_impls )*
		#impl_dispatch_view_function
		#impl_view_function_metadata
	}
}

fn expand_view_function_prefix_impl(
	def: &Def,
	span: Span,
	where_clause: Option<&syn::WhereClause>,
) -> TokenStream {
	let pezpallet_ident = &def.pezpallet_struct.pezpallet;
	let pezframe_support = &def.pezframe_support;
	let pezframe_system = &def.pezframe_system;
	let type_impl_gen = &def.type_impl_generics(span);
	let type_use_gen = &def.type_use_generics(span);

	quote::quote! {
		impl<#type_impl_gen> #pezframe_support::view_functions::ViewFunctionIdPrefix for #pezpallet_ident<#type_use_gen> #where_clause {
			fn prefix() -> [::core::primitive::u8; 16usize] {
				<
					<T as #pezframe_system::Config>::PalletInfo
					as #pezframe_support::traits::PalletInfo
				>::name_hash::<Pezpallet<#type_use_gen>>()
					.expect("No name_hash found for the pezpallet in the runtime! This usually means that the pezpallet wasn't added to `construct_runtime!`.")
			}
		}
	}
}

fn expand_view_function(
	def: &Def,
	span: Span,
	where_clause: Option<&syn::WhereClause>,
	view_fn: &ViewFunctionDef,
) -> TokenStream {
	let pezframe_support = &def.pezframe_support;
	let pezpallet_ident = &def.pezpallet_struct.pezpallet;
	let type_impl_gen = &def.type_impl_generics(span);
	let type_decl_bounded_gen = &def.type_decl_bounded_generics(span);
	let type_use_gen = &def.type_use_generics(span);
	let capture_docs = if cfg!(feature = "no-metadata-docs") { "never" } else { "always" };

	let view_function_struct_ident = view_fn.view_function_struct_ident();
	let view_fn_name = &view_fn.name;
	let (arg_names, arg_types) = match view_fn.args_names_types() {
		Ok((arg_names, arg_types)) => (arg_names, arg_types),
		Err(e) => return e.into_compile_error(),
	};
	let return_type = &view_fn.return_type;
	let docs = &view_fn.docs;

	let view_function_id_suffix_bytes_raw = match view_fn.view_function_id_suffix_bytes() {
		Ok(view_function_id_suffix_bytes_raw) => view_function_id_suffix_bytes_raw,
		Err(e) => return e.into_compile_error(),
	};
	let view_function_id_suffix_bytes = view_function_id_suffix_bytes_raw
		.map(|byte| syn::LitInt::new(&format!("0x{:X}_u8", byte), Span::call_site()));

	quote::quote! {
		#( #[doc = #docs] )*
		#[allow(missing_docs)]
		#[derive(
			#pezframe_support::RuntimeDebugNoBound,
			#pezframe_support::CloneNoBound,
			#pezframe_support::EqNoBound,
			#pezframe_support::PartialEqNoBound,
			#pezframe_support::__private::codec::Encode,
			#pezframe_support::__private::codec::Decode,
			#pezframe_support::__private::codec::DecodeWithMemTracking,
			#pezframe_support::__private::scale_info::TypeInfo,
		)]
		#[codec(encode_bound())]
		#[codec(decode_bound())]
		#[scale_info(skip_type_params(#type_use_gen), capture_docs = #capture_docs)]
		pub struct #view_function_struct_ident<#type_decl_bounded_gen> #where_clause {
			#(
				pub #arg_names: #arg_types,
			)*
			_marker: ::core::marker::PhantomData<(#type_use_gen,)>,
		}

		impl<#type_impl_gen> #view_function_struct_ident<#type_use_gen> #where_clause {
			/// Create a new [`#view_function_struct_ident`] instance.
			pub fn new(#( #arg_names: #arg_types, )*) -> Self {
				Self {
					#( #arg_names, )*
					_marker: ::core::default::Default::default()
				}
			}
		}

		impl<#type_impl_gen> #pezframe_support::view_functions::ViewFunctionIdSuffix for #view_function_struct_ident<#type_use_gen> #where_clause {
			const SUFFIX: [::core::primitive::u8; 16usize] = [ #( #view_function_id_suffix_bytes ),* ];
		}

		impl<#type_impl_gen> #pezframe_support::view_functions::ViewFunction for #view_function_struct_ident<#type_use_gen> #where_clause {
			fn id() -> #pezframe_support::view_functions::ViewFunctionId {
				#pezframe_support::view_functions::ViewFunctionId {
					prefix: <#pezpallet_ident<#type_use_gen> as #pezframe_support::view_functions::ViewFunctionIdPrefix>::prefix(),
					suffix: <Self as #pezframe_support::view_functions::ViewFunctionIdSuffix>::SUFFIX,
				}
			}

			type ReturnType = #return_type;

			fn invoke(self) -> Self::ReturnType {
				let Self { #( #arg_names, )* _marker } = self;
				#pezpallet_ident::<#type_use_gen> :: #view_fn_name( #( #arg_names, )* )
			}
		}
	}
}

fn impl_dispatch_view_function(
	def: &Def,
	span: Span,
	where_clause: Option<&syn::WhereClause>,
	view_fns: &[ViewFunctionDef],
) -> TokenStream {
	let pezframe_support = &def.pezframe_support;
	let pezpallet_ident = &def.pezpallet_struct.pezpallet;
	let type_impl_gen = &def.type_impl_generics(span);
	let type_use_gen = &def.type_use_generics(span);

	let query_match_arms = view_fns.iter().map(|view_fn| {
		let view_function_struct_ident = view_fn.view_function_struct_ident();
		quote::quote! {
			<#view_function_struct_ident<#type_use_gen> as #pezframe_support::view_functions::ViewFunctionIdSuffix>::SUFFIX => {
				<#view_function_struct_ident<#type_use_gen> as #pezframe_support::view_functions::ViewFunction>::execute(input, output)
			}
		}
	});

	quote::quote! {
		impl<#type_impl_gen> #pezframe_support::view_functions::DispatchViewFunction
			for #pezpallet_ident<#type_use_gen> #where_clause
		{
			#[deny(unreachable_patterns)]
			fn dispatch_view_function<O: #pezframe_support::__private::codec::Output>(
				id: & #pezframe_support::view_functions::ViewFunctionId,
				input: &mut &[u8],
				output: &mut O
			) -> Result<(), #pezframe_support::view_functions::ViewFunctionDispatchError>
			{
				match id.suffix {
					#( #query_match_arms )*
					_ => Err(#pezframe_support::view_functions::ViewFunctionDispatchError::NotFound(id.clone())),
				}
			}
		}
	}
}

fn impl_view_function_metadata(
	def: &Def,
	span: Span,
	where_clause: Option<&syn::WhereClause>,
	view_fns: &[ViewFunctionDef],
) -> TokenStream {
	let pezframe_support = &def.pezframe_support;
	let pezpallet_ident = &def.pezpallet_struct.pezpallet;
	let type_impl_gen = &def.type_impl_generics(span);
	let type_use_gen = &def.type_use_generics(span);

	let view_functions = view_fns.iter().map(|view_fn| {
		let view_function_struct_ident = view_fn.view_function_struct_ident();
		let name = &view_fn.name;
		let inputs = view_fn.args.iter().filter_map(|fn_arg| {
			match fn_arg {
				syn::FnArg::Receiver(_) => None,
				syn::FnArg::Typed(typed) => {
					let pat = &typed.pat;
					let ty = &typed.ty;
					Some(quote::quote! {
						#pezframe_support::__private::metadata_ir::PalletViewFunctionParamMetadataIR {
							name: ::core::stringify!(#pat),
							ty: #pezframe_support::__private::scale_info::meta_type::<#ty>(),
						}
					})
				}
			}
		});

		let no_docs = vec![];
		let doc = if cfg!(feature = "no-metadata-docs") { &no_docs } else { &view_fn.docs };

		let deprecation = match crate::deprecation::get_deprecation(
			&quote::quote! { #pezframe_support },
			&def.item.attrs,
		) {
			Ok(deprecation) => deprecation,
			Err(e) => return e.into_compile_error(),
		};

		quote::quote! {
			#pezframe_support::__private::metadata_ir::PalletViewFunctionMetadataIR {
				name: ::core::stringify!(#name),
				id: <#view_function_struct_ident<#type_use_gen> as #pezframe_support::view_functions::ViewFunction>::id().into(),
				inputs: #pezframe_support::__private::pezsp_std::vec![ #( #inputs ),* ],
				output: #pezframe_support::__private::scale_info::meta_type::<
					<#view_function_struct_ident<#type_use_gen> as #pezframe_support::view_functions::ViewFunction>::ReturnType
				>(),
				docs: #pezframe_support::__private::pezsp_std::vec![ #( #doc ),* ],
				deprecation_info: #deprecation,
			}
		}
	});

	quote::quote! {
		impl<#type_impl_gen> #pezpallet_ident<#type_use_gen> #where_clause {
			#[doc(hidden)]
			pub fn pezpallet_view_functions_metadata()
				-> #pezframe_support::__private::Vec<#pezframe_support::__private::metadata_ir::PalletViewFunctionMetadataIR> {
				#pezframe_support::__private::vec![ #( #view_functions ),* ]
			}
		}
	}
}
