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

#[cfg(feature = "decode")]
use codec::Decode;
#[cfg(feature = "serde_full")]
use serde::Serialize;

use super::RuntimeMetadataPrefixed;
use codec::Encode;
use scale_info::prelude::vec::Vec;
use scale_info::{
	form::{Form, MetaForm, PortableForm},
	IntoPortable, MetaType, PortableRegistry, Registry,
};

/// Current prefix of metadata
pub const META_RESERVED: u32 = 0x6174656d; // 'meta' warn endianness

/// Latest runtime metadata
pub type RuntimeMetadataLastVersion = RuntimeMetadataV14;

impl From<RuntimeMetadataLastVersion> for super::RuntimeMetadataPrefixed {
	fn from(metadata: RuntimeMetadataLastVersion) -> RuntimeMetadataPrefixed {
		RuntimeMetadataPrefixed(META_RESERVED, super::RuntimeMetadata::V14(metadata))
	}
}

/// The metadata of a runtime.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
pub struct RuntimeMetadataV14 {
	/// Type registry containing all types used in the metadata.
	pub types: PortableRegistry,
	/// Metadata of all the pezpallets.
	pub pezpallets: Vec<PezpalletMetadata<PortableForm>>,
	/// Metadata of the extrinsic.
	pub extrinsic: ExtrinsicMetadata<PortableForm>,
	/// The type of the `Runtime`.
	pub ty: <PortableForm as Form>::Type,
}

impl RuntimeMetadataV14 {
	/// Create a new instance of [`RuntimeMetadataV14`].
	pub fn new(
		pezpallets: Vec<PezpalletMetadata>,
		extrinsic: ExtrinsicMetadata,
		runtime_type: MetaType,
	) -> Self {
		let mut registry = Registry::new();
		let pezpallets = registry.map_into_portable(pezpallets);
		let extrinsic = extrinsic.into_portable(&mut registry);
		let ty = registry.register_type(&runtime_type);
		Self { types: registry.into(), pezpallets, extrinsic, ty }
	}
}

/// Metadata of the extrinsic used by the runtime.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct ExtrinsicMetadata<T: Form = MetaForm> {
	/// The type of the extrinsic.
	pub ty: T::Type,
	/// Extrinsic version.
	pub version: u8,
	/// The signed extensions in the order they appear in the extrinsic.
	pub signed_extensions: Vec<SignedExtensionMetadata<T>>,
}

impl IntoPortable for ExtrinsicMetadata {
	type Output = ExtrinsicMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		ExtrinsicMetadata {
			ty: registry.register_type(&self.ty),
			version: self.version,
			signed_extensions: registry.map_into_portable(self.signed_extensions),
		}
	}
}

/// Metadata of an extrinsic's signed extension.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct SignedExtensionMetadata<T: Form = MetaForm> {
	/// The unique signed extension identifier, which may be different from the type name.
	pub identifier: T::String,
	/// The type of the signed extension, with the data to be included in the extrinsic.
	pub ty: T::Type,
	/// The type of the additional signed data, with the data to be included in the signed payload
	pub additional_signed: T::Type,
}

impl IntoPortable for SignedExtensionMetadata {
	type Output = SignedExtensionMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		SignedExtensionMetadata {
			identifier: self.identifier.into_portable(registry),
			ty: registry.register_type(&self.ty),
			additional_signed: registry.register_type(&self.additional_signed),
		}
	}
}

/// All metadata about an runtime pezpallet.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct PezpalletMetadata<T: Form = MetaForm> {
	/// Pezpallet name.
	pub name: T::String,
	/// Pezpallet storage metadata.
	pub storage: Option<PezpalletStorageMetadata<T>>,
	/// Pezpallet calls metadata.
	pub calls: Option<PezpalletCallMetadata<T>>,
	/// Pezpallet event metadata.
	pub event: Option<PezpalletEventMetadata<T>>,
	/// Pezpallet constants metadata.
	pub constants: Vec<PezpalletConstantMetadata<T>>,
	/// Pezpallet error metadata.
	pub error: Option<PezpalletErrorMetadata<T>>,
	/// Define the index of the pezpallet, this index will be used for the encoding of pezpallet event,
	/// call and origin variants.
	pub index: u8,
}

impl IntoPortable for PezpalletMetadata {
	type Output = PezpalletMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		PezpalletMetadata {
			name: self.name.into_portable(registry),
			storage: self.storage.map(|storage| storage.into_portable(registry)),
			calls: self.calls.map(|calls| calls.into_portable(registry)),
			event: self.event.map(|event| event.into_portable(registry)),
			constants: registry.map_into_portable(self.constants),
			error: self.error.map(|error| error.into_portable(registry)),
			index: self.index,
		}
	}
}

/// All metadata of the pezpallet's storage.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct PezpalletStorageMetadata<T: Form = MetaForm> {
	/// The common prefix used by all storage entries.
	pub prefix: T::String,
	/// Metadata for all storage entries.
	pub entries: Vec<StorageEntryMetadata<T>>,
}

impl IntoPortable for PezpalletStorageMetadata {
	type Output = PezpalletStorageMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		PezpalletStorageMetadata {
			prefix: self.prefix.into_portable(registry),
			entries: registry.map_into_portable(self.entries),
		}
	}
}

/// Metadata about one storage entry.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct StorageEntryMetadata<T: Form = MetaForm> {
	/// Variable name of the storage entry.
	pub name: T::String,
	/// An `Option` modifier of that storage entry.
	pub modifier: StorageEntryModifier,
	/// Type of the value stored in the entry.
	pub ty: StorageEntryType<T>,
	/// Default value (SCALE encoded).
	pub default: Vec<u8>,
	/// Storage entry documentation.
	pub docs: Vec<T::String>,
}

impl IntoPortable for StorageEntryMetadata {
	type Output = StorageEntryMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		StorageEntryMetadata {
			name: self.name.into_portable(registry),
			modifier: self.modifier,
			ty: self.ty.into_portable(registry),
			default: self.default,
			docs: registry.map_into_portable(self.docs),
		}
	}
}

/// A storage entry modifier indicates how a storage entry is returned when fetched and what the value will be if the key is not present.
/// Specifically this refers to the "return type" when fetching a storage entry, and what the value will be if the key is not present.
///
/// `Optional` means you should expect an `Option<T>`, with `None` returned if the key is not present.
/// `Default` means you should expect a `T` with the default value of default if the key is not present.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
pub enum StorageEntryModifier {
	/// The storage entry returns an `Option<T>`, with `None` if the key is not present.
	Optional,
	/// The storage entry returns `T::Default` if the key is not present.
	Default,
}

/// Hasher used by storage maps
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
pub enum StorageHasher {
	/// 128-bit Blake2 hash.
	Blake2_128,
	/// 256-bit Blake2 hash.
	Blake2_256,
	/// Multiple 128-bit Blake2 hashes concatenated.
	Blake2_128Concat,
	/// 128-bit XX hash.
	Twox128,
	/// 256-bit XX hash.
	Twox256,
	/// Multiple 64-bit XX hashes concatenated.
	Twox64Concat,
	/// Identity hashing (no hashing).
	Identity,
}

/// A type of storage value.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub enum StorageEntryType<T: Form = MetaForm> {
	/// Plain storage entry (just the value).
	Plain(T::Type),
	/// A storage map.
	Map {
		/// One or more hashers, should be one hasher per key element.
		hashers: Vec<StorageHasher>,
		/// The type of the key, can be a tuple with elements for each of the hashers.
		key: T::Type,
		/// The type of the value.
		value: T::Type,
	},
}

impl IntoPortable for StorageEntryType {
	type Output = StorageEntryType<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		match self {
			Self::Plain(plain) => StorageEntryType::Plain(registry.register_type(&plain)),
			Self::Map { hashers, key, value } => StorageEntryType::Map {
				hashers,
				key: registry.register_type(&key),
				value: registry.register_type(&value),
			},
		}
	}
}

/// Metadata for all calls in a pezpallet
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct PezpalletCallMetadata<T: Form = MetaForm> {
	/// The corresponding enum type for the pezpallet call.
	pub ty: T::Type,
}

impl IntoPortable for PezpalletCallMetadata {
	type Output = PezpalletCallMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		PezpalletCallMetadata { ty: registry.register_type(&self.ty) }
	}
}

impl From<MetaType> for PezpalletCallMetadata {
	fn from(ty: MetaType) -> Self {
		Self { ty }
	}
}

/// Metadata about the pezpallet Event type.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
pub struct PezpalletEventMetadata<T: Form = MetaForm> {
	/// The Event type.
	pub ty: T::Type,
}

impl IntoPortable for PezpalletEventMetadata {
	type Output = PezpalletEventMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		PezpalletEventMetadata { ty: registry.register_type(&self.ty) }
	}
}

impl From<MetaType> for PezpalletEventMetadata {
	fn from(ty: MetaType) -> Self {
		Self { ty }
	}
}

/// Metadata about one pezpallet constant.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(
	feature = "serde_full",
	serde(bound(serialize = "T::Type: Serialize, T::String: Serialize"))
)]
pub struct PezpalletConstantMetadata<T: Form = MetaForm> {
	/// Name of the pezpallet constant.
	pub name: T::String,
	/// Type of the pezpallet constant.
	pub ty: T::Type,
	/// Value stored in the constant (SCALE encoded).
	pub value: Vec<u8>,
	/// Documentation of the constant.
	pub docs: Vec<T::String>,
}

impl IntoPortable for PezpalletConstantMetadata {
	type Output = PezpalletConstantMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		PezpalletConstantMetadata {
			name: self.name.into_portable(registry),
			ty: registry.register_type(&self.ty),
			value: self.value,
			docs: registry.map_into_portable(self.docs),
		}
	}
}

/// Metadata about a pezpallet error.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
#[cfg_attr(feature = "serde_full", serde(bound(serialize = "T::Type: Serialize")))]
pub struct PezpalletErrorMetadata<T: Form = MetaForm> {
	/// The error type information.
	pub ty: T::Type,
}

impl IntoPortable for PezpalletErrorMetadata {
	type Output = PezpalletErrorMetadata<PortableForm>;

	fn into_portable(self, registry: &mut Registry) -> Self::Output {
		PezpalletErrorMetadata { ty: registry.register_type(&self.ty) }
	}
}

impl From<MetaType> for PezpalletErrorMetadata {
	fn from(ty: MetaType) -> Self {
		Self { ty }
	}
}
