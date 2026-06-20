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

use syn::parse_quote;

#[test]
fn test_parse_pallet_with_task_enum_missing_impl() {
	assert_pallet_parse_error! {
		#[manifest_dir("../../examples/basic")]
		#[error_regex("Missing `\\#\\[pezpallet::tasks_experimental\\]` impl")]
		#[pezframe_support::pezpallet]
		pub mod pezpallet {
			#[pezpallet::task_enum]
			pub enum Task<T: Config> {
				Something,
			}

			#[pezpallet::config]
			pub trait Config: pezframe_system::Config {}

			#[pezpallet::pezpallet]
			pub struct Pezpallet<T>(_);
		}
	}
}

#[test]
fn test_parse_pallet_with_task_enum_wrong_attribute() {
	assert_pallet_parse_error! {
		#[manifest_dir("../../examples/basic")]
		#[error_regex("expected one of")]
		#[pezframe_support::pezpallet]
		pub mod pezpallet {
			#[pezpallet::wrong_attribute]
			pub enum Task<T: Config> {
				Something,
			}

			#[pezpallet::task_list]
			impl<T: Config> pezframe_support::traits::Task for Task<T>
			where
				T: TypeInfo,
			{}

			#[pezpallet::config]
			pub trait Config: pezframe_system::Config {}

			#[pezpallet::pezpallet]
			pub struct Pezpallet<T>(_);
		}
	}
}

#[test]
fn test_parse_pallet_missing_task_enum() {
	assert_pallet_parses! {
		#[manifest_dir("../../examples/basic")]
		#[pezframe_support::pezpallet]
		pub mod pezpallet {
			#[pezpallet::tasks_experimental]
			#[cfg(test)] // aha, this means it's being eaten
			impl<T: Config> pezframe_support::traits::Task for Task<T>
			where
				T: TypeInfo,
			{}

			#[pezpallet::config]
			pub trait Config: pezframe_system::Config {}

			#[pezpallet::pezpallet]
			pub struct Pezpallet<T>(_);
		}
	};
}

#[test]
fn test_parse_pallet_task_list_in_wrong_place() {
	assert_pallet_parse_error! {
		#[manifest_dir("../../examples/basic")]
		#[error_regex("can only be used on items within an `impl` statement.")]
		#[pezframe_support::pezpallet]
		pub mod pezpallet {
			pub enum MyCustomTaskEnum<T: Config> {
				Something,
			}

			#[pezpallet::task_list]
			pub fn something() {
				println!("hey");
			}

			#[pezpallet::config]
			pub trait Config: pezframe_system::Config {}

			#[pezpallet::pezpallet]
			pub struct Pezpallet<T>(_);
		}
	}
}

#[test]
fn test_parse_pallet_manual_tasks_impl_without_manual_tasks_enum() {
	assert_pallet_parse_error! {
		#[manifest_dir("../../examples/basic")]
		#[error_regex(".*attribute must be attached to your.*")]
		#[pezframe_support::pezpallet]
		pub mod pezpallet {

			impl<T: Config> pezframe_support::traits::Task for Task<T>
			where
				T: TypeInfo,
			{
				type Enumeration = alloc::vec::IntoIter<Task<T>>;

				fn iter() -> Self::Enumeration {
					alloc::vec![Task::increment, Task::decrement].into_iter()
				}
			}

			#[pezpallet::config]
			pub trait Config: pezframe_system::Config {}

			#[pezpallet::pezpallet]
			pub struct Pezpallet<T>(_);
		}
	}
}

#[test]
fn test_parse_pallet_manual_task_enum_non_manual_impl() {
	assert_pallet_parses! {
		#[manifest_dir("../../examples/basic")]
		#[pezframe_support::pezpallet]
		pub mod pezpallet {
			pub enum MyCustomTaskEnum<T: Config> {
				Something,
			}

			#[pezpallet::tasks_experimental]
			impl<T: Config> pezframe_support::traits::Task for MyCustomTaskEnum<T>
			where
				T: TypeInfo,
			{}

			#[pezpallet::config]
			pub trait Config: pezframe_system::Config {}

			#[pezpallet::pezpallet]
			pub struct Pezpallet<T>(_);
		}
	};
}

#[test]
fn test_parse_pallet_non_manual_task_enum_manual_impl() {
	assert_pallet_parses! {
		#[manifest_dir("../../examples/basic")]
		#[pezframe_support::pezpallet]
		pub mod pezpallet {
			#[pezpallet::task_enum]
			pub enum MyCustomTaskEnum<T: Config> {
				Something,
			}

			impl<T: Config> pezframe_support::traits::Task for MyCustomTaskEnum<T>
			where
				T: TypeInfo,
			{}

			#[pezpallet::config]
			pub trait Config: pezframe_system::Config {}

			#[pezpallet::pezpallet]
			pub struct Pezpallet<T>(_);
		}
	};
}

#[test]
fn test_parse_pallet_manual_task_enum_manual_impl() {
	assert_pallet_parses! {
		#[manifest_dir("../../examples/basic")]
		#[pezframe_support::pezpallet]
		pub mod pezpallet {
			pub enum MyCustomTaskEnum<T: Config> {
				Something,
			}

			impl<T: Config> pezframe_support::traits::Task for MyCustomTaskEnum<T>
			where
				T: TypeInfo,
			{}

			#[pezpallet::config]
			pub trait Config: pezframe_system::Config {}

			#[pezpallet::pezpallet]
			pub struct Pezpallet<T>(_);
		}
	};
}

#[test]
fn test_parse_pallet_manual_task_enum_mismatch_ident() {
	assert_pallet_parses! {
		#[manifest_dir("../../examples/basic")]
		#[pezframe_support::pezpallet]
		pub mod pezpallet {
			pub enum WrongIdent<T: Config> {
				Something,
			}

			#[pezpallet::tasks_experimental]
			impl<T: Config> pezframe_support::traits::Task for MyCustomTaskEnum<T>
			where
				T: TypeInfo,
			{}

			#[pezpallet::config]
			pub trait Config: pezframe_system::Config {}

			#[pezpallet::pezpallet]
			pub struct Pezpallet<T>(_);
		}
	};
}
