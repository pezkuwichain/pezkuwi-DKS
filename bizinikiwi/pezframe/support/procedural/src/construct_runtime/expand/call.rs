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

pub fn expand_outer_dispatch(
	runtime: &Ident,
	system_pallet: &Pezpallet,
	pezpallet_decls: &[Pezpallet],
	scrate: &TokenStream,
) -> TokenStream {
	let mut variant_defs = TokenStream::new();
	let mut variant_patterns = Vec::new();
	let mut variant_usages = Vec::new();
	let mut query_call_part_macros = Vec::new();
	let mut pezpallet_names = Vec::new();
	let mut pezpallet_attrs = Vec::new();
	let system_path = &system_pallet.path;

	let pallets_with_call = pezpallet_decls.iter().filter(|decl| decl.exists_part("Call"));

	for pezpallet_declaration in pallets_with_call {
		let name = &pezpallet_declaration.name;
		let path = &pezpallet_declaration.path;
		let index = pezpallet_declaration.index;
		let attr = pezpallet_declaration.get_attributes();

		variant_defs.extend(quote! {
			#attr
			#[codec(index = #index)]
			#name( #scrate::dispatch::CallableCallFor<#name, #runtime> ),
		});
		variant_usages.push(quote!( #scrate::dispatch::CallableCallFor<#name, #runtime> ));
		variant_patterns.push(quote!(RuntimeCall::#name(call)));
		pezpallet_names.push(name);
		pezpallet_attrs.push(attr);
		query_call_part_macros.push(quote! {
			#path::__bizinikiwi_call_check::is_call_part_defined!(#name);
		});
	}

	quote! {
		#( #query_call_part_macros )*

		/// The aggregated runtime call type.
		#[derive(
			Clone, PartialEq, Eq,
			#scrate::__private::codec::Encode,
			#scrate::__private::codec::Decode,
			#scrate::__private::codec::DecodeWithMemTracking,
			#scrate::__private::scale_info::TypeInfo,
			#scrate::__private::RuntimeDebug,
		)]
		pub enum RuntimeCall {
			#variant_defs
		}
		#[cfg(test)]
		impl RuntimeCall {
			/// Return a list of the module names together with their size in memory.
			pub const fn sizes() -> &'static [( &'static str, usize )] {
				use #scrate::dispatch::Callable;
				use core::mem::size_of;
				&[#(
					#pezpallet_attrs
					(
						stringify!(#pezpallet_names),
						size_of::< <#pezpallet_names as Callable<#runtime>>::RuntimeCall >(),
					),
				)*]
			}

			/// Panics with diagnostic information if the size is greater than the given `limit`.
			pub fn assert_size_under(limit: usize) {
				let size = core::mem::size_of::<Self>();
				let call_oversize = size > limit;
				if call_oversize {
					println!("Size of `Call` is {} bytes (provided limit is {} bytes)", size, limit);
					let mut sizes = Self::sizes().to_vec();
					sizes.sort_by_key(|x| -(x.1 as isize));
					for (i, &(name, size)) in sizes.iter().enumerate().take(5) {
						println!("Offender #{}: {} at {} bytes", i + 1, name, size);
					}
					if let Some((_, next_size)) = sizes.get(5) {
						println!("{} others of size {} bytes or less", sizes.len() - 5, next_size);
					}
					panic!(
						"Size of `Call` is more than limit; use `Box` on complex parameter types to reduce the
						size of `Call`.
						If the limit is too strong, maybe consider providing a higher limit."
					);
				}
			}
		}
		impl #scrate::dispatch::GetDispatchInfo for RuntimeCall {
			fn get_dispatch_info(&self) -> #scrate::dispatch::DispatchInfo {
				match self {
					#(
						#pezpallet_attrs
						#variant_patterns => call.get_dispatch_info(),
					)*
				}
			}
		}

		impl #scrate::dispatch::CheckIfFeeless for RuntimeCall {
			type Origin = #system_path::pezpallet_prelude::OriginFor<#runtime>;
			fn is_feeless(&self, origin: &Self::Origin) -> bool {
				match self {
					#(
						#pezpallet_attrs
						#variant_patterns => call.is_feeless(origin),
					)*
				}
			}
		}

		impl #scrate::traits::GetCallMetadata for RuntimeCall {
			fn get_call_metadata(&self) -> #scrate::traits::CallMetadata {
				use #scrate::traits::GetCallName;
				match self {
					#(
						#pezpallet_attrs
						#variant_patterns => {
							let function_name = call.get_call_name();
							let pezpallet_name = stringify!(#pezpallet_names);
							#scrate::traits::CallMetadata { function_name, pezpallet_name }
						}
					)*
				}
			}

			fn get_module_names() -> &'static [&'static str] {
				&[#(
					#pezpallet_attrs
					stringify!(#pezpallet_names),
				)*]
			}

			fn get_call_names(module: &str) -> &'static [&'static str] {
				use #scrate::{dispatch::Callable, traits::GetCallName};
				match module {
					#(
						#pezpallet_attrs
						stringify!(#pezpallet_names) =>
							<<#pezpallet_names as Callable<#runtime>>::RuntimeCall
								as GetCallName>::get_call_names(),
					)*
					_ => unreachable!(),
				}
			}
		}
		impl #scrate::__private::Dispatchable for RuntimeCall {
			type RuntimeOrigin = RuntimeOrigin;
			type Config = RuntimeCall;
			type Info = #scrate::dispatch::DispatchInfo;
			type PostInfo = #scrate::dispatch::PostDispatchInfo;
			fn dispatch(self, origin: RuntimeOrigin) -> #scrate::dispatch::DispatchResultWithPostInfo {
				if !<Self::RuntimeOrigin as #scrate::traits::OriginTrait>::filter_call(&origin, &self) {
					return ::core::result::Result::Err(
						#system_path::Error::<#runtime>::CallFiltered.into()
					);
				}

				#scrate::traits::UnfilteredDispatchable::dispatch_bypass_filter(self, origin)
			}
		}
		impl #scrate::traits::UnfilteredDispatchable for RuntimeCall {
			type RuntimeOrigin = RuntimeOrigin;
			fn dispatch_bypass_filter(self, origin: RuntimeOrigin) -> #scrate::dispatch::DispatchResultWithPostInfo {
				match self {
					#(
						#pezpallet_attrs
						#variant_patterns =>
							#scrate::traits::UnfilteredDispatchable::dispatch_bypass_filter(call, origin),
					)*
				}
			}
		}

		#(
			#pezpallet_attrs
			impl #scrate::traits::IsSubType<#scrate::dispatch::CallableCallFor<#pezpallet_names, #runtime>> for RuntimeCall {
				#[allow(unreachable_patterns)]
				fn is_sub_type(&self) -> Option<&#scrate::dispatch::CallableCallFor<#pezpallet_names, #runtime>> {
					match self {
						#variant_patterns => Some(call),
						// May be unreachable
						_ => None,
					}
				}
			}

			#pezpallet_attrs
			impl From<#scrate::dispatch::CallableCallFor<#pezpallet_names, #runtime>> for RuntimeCall {
				fn from(call: #scrate::dispatch::CallableCallFor<#pezpallet_names, #runtime>) -> Self {
					#variant_patterns
				}
			}
		)*

		impl #scrate::traits::Authorize for RuntimeCall {
			fn authorize(
				&self,
				source: #scrate::pezpallet_prelude::TransactionSource,
			) -> ::core::option::Option<
				::core::result::Result<
					(
						#scrate::pezpallet_prelude::ValidTransaction,
						#scrate::pezpallet_prelude::Weight,
					),
					#scrate::pezpallet_prelude::TransactionValidityError
				>
			> {
				match self {
					#(
						#pezpallet_attrs
						#variant_patterns => #scrate::traits::Authorize::authorize(call, source),
					)*
				}
			}

			fn weight_of_authorize(&self) -> #scrate::pezpallet_prelude::Weight {
				match self {
					#(
						#pezpallet_attrs
						#variant_patterns =>
							#scrate::traits::Authorize::weight_of_authorize(call),
					)*
				}
			}
		}
	}
}
