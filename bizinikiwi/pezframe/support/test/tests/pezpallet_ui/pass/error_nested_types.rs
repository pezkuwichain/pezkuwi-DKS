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

use codec::{Decode, DecodeWithMemTracking, Encode};
use pezframe_support::PalletError;

#[pezframe_support::pezpallet]
#[allow(unused_imports)]
pub mod pezpallet {
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(core::marker::PhantomData<T>);

	#[pezpallet::error]
	pub enum Error<T> {
		CustomError(crate::MyError),
	}
}

#[derive(Encode, Decode, DecodeWithMemTracking, PalletError, scale_info::TypeInfo)]
pub enum MyError {
	Foo,
	Bar,
	Baz(NestedError),
	Struct(MyStruct),
	Wrapper(Wrapper),
}

#[derive(Encode, Decode, DecodeWithMemTracking, PalletError, scale_info::TypeInfo)]
pub enum NestedError {
	Quux,
}

#[derive(Encode, Decode, DecodeWithMemTracking, PalletError, scale_info::TypeInfo)]
pub struct MyStruct {
	field: u8,
}

#[derive(Encode, Decode, DecodeWithMemTracking, PalletError, scale_info::TypeInfo)]
pub struct Wrapper(bool);

fn main() {}
