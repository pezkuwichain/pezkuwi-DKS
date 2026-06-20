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
// limitations under the License

use crate::construct_runtime::Pezpallet;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn expand_outer_validate_unsigned(
	runtime: &Ident,
	pezpallet_decls: &[Pezpallet],
	scrate: &TokenStream,
) -> TokenStream {
	let mut pezpallet_names = Vec::new();
	let mut pezpallet_attrs = Vec::new();
	let mut query_validate_unsigned_part_macros = Vec::new();

	for pezpallet_decl in pezpallet_decls {
		if pezpallet_decl.exists_part("ValidateUnsigned") {
			let name = &pezpallet_decl.name;
			let path = &pezpallet_decl.path;
			let attr = pezpallet_decl.get_attributes();

			pezpallet_names.push(name);
			pezpallet_attrs.push(attr);
			query_validate_unsigned_part_macros.push(quote! {
				#path::__bizinikiwi_validate_unsigned_check::is_validate_unsigned_part_defined!(#name);
			});
		}
	}

	quote! {
		#( #query_validate_unsigned_part_macros )*

		impl #scrate::unsigned::ValidateUnsigned for #runtime {
			type Call = RuntimeCall;

			fn pre_dispatch(call: &Self::Call) -> Result<(), #scrate::unsigned::TransactionValidityError> {
				#[allow(unreachable_patterns)]
				match call {
					#(
						#pezpallet_attrs
						RuntimeCall::#pezpallet_names(inner_call) => #pezpallet_names::pre_dispatch(inner_call),
					)*
					// pre-dispatch should not stop inherent extrinsics, validation should prevent
					// including arbitrary (non-inherent) extrinsics to blocks.
					_ => Ok(()),
				}
			}

			fn validate_unsigned(
				#[allow(unused_variables)]
				source: #scrate::unsigned::TransactionSource,
				call: &Self::Call,
			) -> #scrate::unsigned::TransactionValidity {
				#[allow(unreachable_patterns)]
				match call {
					#(
						#pezpallet_attrs
						RuntimeCall::#pezpallet_names(inner_call) => #pezpallet_names::validate_unsigned(source, inner_call),
					)*
					_ => #scrate::unsigned::UnknownTransaction::NoUnsignedValidator.into(),
				}
			}
		}
	}
}
