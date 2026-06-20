// Copyright (C) 2021-2022 Parity Technologies (UK) Ltd.
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

use super::{Ss58AddressFormat, Ss58AddressFormatRegistry, TokenRegistry};

#[test]
fn is_reserved() {
	let reserved: Ss58AddressFormat = Ss58AddressFormatRegistry::Reserved46Account.into();
	assert!(reserved.is_reserved());

	let not_reserved: Ss58AddressFormat = Ss58AddressFormatRegistry::PolkadexAccount.into();
	assert!(!not_reserved.is_reserved());

	assert!(!Ss58AddressFormat::custom(100).is_reserved());
}

#[test]
fn is_custom() {
	assert!(Ss58AddressFormat::custom(432).is_custom());

	let reserved: Ss58AddressFormat = Ss58AddressFormatRegistry::Reserved46Account.into();
	assert!(!reserved.is_custom());

	let not_reserved: Ss58AddressFormat = Ss58AddressFormatRegistry::PolkadexAccount.into();
	assert!(!not_reserved.is_custom());
}

#[cfg(feature = "std")]
#[test]
fn enum_to_name_and_back() {
	use std::convert::TryInto;
	for name in Ss58AddressFormat::all_names() {
		let val: Ss58AddressFormatRegistry = (*name).try_into().expect(name);
		assert_eq!(name, &val.to_string());

		let val: Ss58AddressFormatRegistry = name.to_lowercase().as_str().try_into().expect(name);
		assert_eq!(name, &val.to_string());

		let val: Ss58AddressFormatRegistry =
			name.to_ascii_uppercase().as_str().try_into().expect(name);
		assert_eq!(name, &val.to_string());
	}
}

#[test]
fn prefix() {
	let hez: Ss58AddressFormat = Ss58AddressFormatRegistry::PezkuwiAccount.into();
	assert_eq!(hez.prefix(), 0);
	let zgr: Ss58AddressFormat = Ss58AddressFormatRegistry::ZagrosAccount.into();
	assert_eq!(zgr.prefix(), 2);
}

#[test]
fn tokens() {
	let pezkuwi = Ss58AddressFormatRegistry::PezkuwiAccount;
	assert_eq!(pezkuwi.tokens(), &[TokenRegistry::Hez]);
	let zagros = Ss58AddressFormatRegistry::ZagrosAccount;
	assert_eq!(zagros.tokens(), &[TokenRegistry::Zgr]);
	let n46 = Ss58AddressFormatRegistry::Reserved46Account;
	assert_eq!(n46.tokens(), &[]);
}
