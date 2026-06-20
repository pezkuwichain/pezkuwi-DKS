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

use pezsp_runtime::traits::Block as BlockT;
use bizinikiwi_test_runtime_client::runtime::Block;

struct Runtime {}

pezsp_api::decl_runtime_apis! {
	#[api_version(2)]
	pub trait Api {
		fn test1();
		fn test2();
		#[api_version(3)]
		fn test3();
	}
}

pezsp_api::impl_runtime_apis! {
	#[api_version(3)]
	impl self::Api<Block> for Runtime {
		fn test1() {}
		fn test2() {}
	}

	impl pezsp_api::Core<Block> for Runtime {
		fn version() -> pezsp_version::RuntimeVersion {
			unimplemented!()
		}
		fn execute_block(_: <Block as BlockT>::LazyBlock) {
			unimplemented!()
		}
		fn initialize_block(_: &<Block as BlockT>::Header) -> pezsp_runtime::ExtrinsicInclusionMode {
			unimplemented!()
		}
	}
}

fn main() {}
