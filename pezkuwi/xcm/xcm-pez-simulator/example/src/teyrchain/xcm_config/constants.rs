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

use crate::teyrchain::Runtime;
use pezframe_support::parameter_types;
use xcm::latest::prelude::*;
use xcm_pez_simulator::mock_message_queue::TeyrchainId;

parameter_types! {
	pub KsmPerSecondPerByte: (AssetId, u128, u128) = (AssetId(Parent.into()), 1, 1);
	pub const MaxAssetsIntoHolding: u32 = 64;
}

parameter_types! {
	pub const KsmLocation: Location = Location::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Dicle;
	pub UniversalLocation: InteriorLocation = [GlobalConsensus(RelayNetwork::get()), Teyrchain(TeyrchainId::<Runtime>::get().into())].into();
}
