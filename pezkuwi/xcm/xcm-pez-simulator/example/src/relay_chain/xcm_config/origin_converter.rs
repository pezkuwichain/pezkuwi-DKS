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

use crate::relay_chain::{
	constants::RelayNetwork, location_converter::LocationConverter, RuntimeOrigin,
};
use pezkuwi_runtime_teyrchains::origin;
use pezkuwi_teyrchain_primitives::primitives::Id as ParaId;
use xcm_builder::{
	ChildSystemTeyrchainAsSuperuser, ChildTeyrchainAsNative, SignedAccountId32AsNative,
	SovereignSignedViaLocation,
};

type LocalOriginConverter = (
	SovereignSignedViaLocation<LocationConverter, RuntimeOrigin>,
	ChildTeyrchainAsNative<origin::Origin, RuntimeOrigin>,
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	ChildSystemTeyrchainAsSuperuser<ParaId, RuntimeOrigin>,
);

pub type OriginConverter = LocalOriginConverter;
