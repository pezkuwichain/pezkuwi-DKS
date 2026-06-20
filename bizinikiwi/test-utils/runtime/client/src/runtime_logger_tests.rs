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

//! Tests for runtime logger integration.
//!
//! These tests verify that logging from within the runtime works correctly
//! with the test client infrastructure.

use crate::{
	runtime::TestAPI, DefaultTestClientBuilderExt, TestClientBuilder, TestClientBuilderExt,
};
use pezsp_api::ProvideRuntimeApi;
use std::env;

#[test]
fn ensure_runtime_logger_works() {
	if env::var("RUN_TEST").is_ok() {
		pezsp_tracing::try_init_simple();

		let client = TestClientBuilder::new().build();
		let runtime_api = client.runtime_api();
		runtime_api
			.do_trace_log(client.chain_info().genesis_hash)
			.expect("Logging should not fail");
	} else {
		for (level, should_print) in &[("test=trace", true), ("info", false)] {
			let executable = std::env::current_exe().unwrap();
			let output = std::process::Command::new(executable)
				.env("RUN_TEST", "1")
				.env("RUST_LOG", level)
				.args(&["--nocapture", "ensure_runtime_logger_works"])
				.output()
				.unwrap();

			let output = String::from_utf8(output.stderr).unwrap();
			assert!(output.contains("Hey I'm runtime") == *should_print);
			assert!(output.contains("THIS IS TRACING") == *should_print);
			assert!(output.contains("Hey, I'm tracing") == *should_print);
		}
	}
}
