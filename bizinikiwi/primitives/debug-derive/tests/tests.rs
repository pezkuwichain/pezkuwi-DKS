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

#[derive(Debug)]
struct Unnamed(u64, String);

#[derive(Debug)]
struct Named {
	a: u64,
	b: String,
}

#[derive(Debug)]
enum EnumLongName<A> {
	A,
	B(A, String),
	VariantLongName { a: A, b: String },
}

/// What `#[derive(Debug)]` prints for the shapes this crate's own macro used to cover.
///
/// The types above moved from `RuntimeDebug` to the standard derive when that macro was
/// deprecated, and the expectations did not move with them: the three enum cases still
/// asserted the `EnumLongName::` prefix, which is the deprecated macro's format and not
/// std's. The structs agreed either way, which is why only half the test was wrong.
///
/// It reads as a test of the standard library because that is now what these types use. The
/// value is in the shapes -- a generic parameter, a unit variant, a tuple variant, a named
/// variant -- which is the set anything replacing the derive has to keep rendering the same.
#[test]
fn should_display_proper_debug() {
	use self::EnumLongName as Enum;

	assert_eq!(format!("{:?}", Unnamed(1, "abc".into())), "Unnamed(1, \"abc\")");
	assert_eq!(format!("{:?}", Named { a: 1, b: "abc".into() }), "Named { a: 1, b: \"abc\" }");
	assert_eq!(format!("{:?}", Enum::<u64>::A), "A");
	assert_eq!(format!("{:?}", Enum::B(1, "abc".into())), "B(1, \"abc\")");
	assert_eq!(
		format!("{:?}", Enum::VariantLongName { a: 1, b: "abc".into() }),
		"VariantLongName { a: 1, b: \"abc\" }"
	);
}
