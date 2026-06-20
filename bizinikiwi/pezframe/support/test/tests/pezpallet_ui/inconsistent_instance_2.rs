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

#[pezframe_support::pezpallet]
mod pezpallet {
	use pezframe_support::pezpallet_prelude::Hooks;
	use pezframe_system::pezpallet_prelude::BlockNumberFor;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T, I = ()>(core::marker::PhantomData<(T, I)>);

	#[pezpallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pezpallet<T, I> {}

	#[pezpallet::call]
	impl<T: Config<I>, I: 'static> Pezpallet<T, I> {}
}

fn main() {
}
