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

use pezframe_support::traits::ContainsPair;
use xcm::latest::{Asset, Location};

/// Filters assets/location pairs.
///
/// Can be amalgamated into tuples. If any item returns `true`, it short-circuits, else `false` is
/// returned.
#[deprecated = "Use `pezframe_support::traits::ContainsPair<Asset, Location>` instead"]
pub trait FilterAssetLocation {
	/// A filter to distinguish between asset/location pairs.
	fn contains(asset: &Asset, origin: &Location) -> bool;
}

#[allow(deprecated)]
impl<T: ContainsPair<Asset, Location>> FilterAssetLocation for T {
	fn contains(asset: &Asset, origin: &Location) -> bool {
		T::contains(asset, origin)
	}
}
