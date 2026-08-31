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

mod aliases;
mod collectives_salary;
mod fellowship;
// `fellowship_treasury` stood here. It funded the Fellowship Treasury from the *relay's*
// treasury, with the relay's `Treasurer` origin, and both were retired when the treasury moved
// to the Asset Hub. The test could not be repaired, only rewritten against a funding path that
// does not exist yet: the Fellowship Treasury on this chain now has no source of funds at all.
// That gap is recorded as an open item rather than left as a test nobody can compile.
mod teleport;
