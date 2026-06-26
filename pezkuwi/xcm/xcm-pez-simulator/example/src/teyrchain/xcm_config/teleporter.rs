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

use pezframe_support::parameter_types;
use xcm::latest::prelude::*;

parameter_types! {
	pub NftCollectionOne: AssetFilter
		= Wild(AllOf { fun: WildNonFungible, id: AssetId((Parent, GeneralIndex(1)).into()) });
	pub NftCollectionOneForRelay: (AssetFilter, Location)
		= (NftCollectionOne::get(), (Parent,).into());
}

pub type TrustedTeleporters = xcm_builder::Case<NftCollectionOneForRelay>;
