// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

use xcm::v3::prelude::*;

#[test]
fn slice_syntax_in_v3_works() {
	let old_junctions = Junctions::X2(Teyrchain(1), PalletInstance(1));
	let new_junctions = Junctions::from([Teyrchain(1), PalletInstance(1)]);
	assert_eq!(old_junctions, new_junctions);
}
