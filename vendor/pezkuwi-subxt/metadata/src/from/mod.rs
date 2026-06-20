// Copyright 2019-2026 Dijital Kurdistan Tech Institute
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

use alloc::string::String;
use core::fmt;
mod v14;
mod v15;
mod v16;

/// Legacy translation hidden behind the corresponding feature flag.
#[cfg(feature = "legacy")]
pub mod legacy;

/// The metadata versions that we support converting into [`crate::Metadata`].
/// These are ordest from highest to lowest, so that the metadata we'd want to
/// pick first is first in the array.
pub const SUPPORTED_METADATA_VERSIONS: [u32; 3] = [16, 15, 14];

/// An error emitted if something goes wrong converting [`frame_metadata`]
/// types into [`crate::Metadata`].
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TryFromError {
	/// Type missing from type registry
	TypeNotFound(u32),
	/// Type was not a variant/enum type
	VariantExpected(u32),
	/// An unsupported metadata version was provided.
	UnsupportedMetadataVersion(u32),
	/// Type name missing from type registry
	TypeNameNotFound(String),
	/// Invalid type path.
	InvalidTypePath(String),
	/// Cannot decode storage entry information.
	StorageInfoError(frame_decode::storage::StorageInfoError<'static>),
	/// Cannot decode Runtime API information.
	RuntimeInfoError(frame_decode::runtime_apis::RuntimeApiInfoError<'static>),
	/// Cannot decode View Function information.
	ViewFunctionInfoError(frame_decode::view_functions::ViewFunctionInfoError<'static>),
}

impl fmt::Display for TryFromError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::TypeNotFound(id) => {
				write!(f, "Type id {id} is expected but not found in the type registry")
			},
			Self::VariantExpected(id) => {
				write!(f, "Type {id} was not a variant/enum type, but is expected to be one")
			},
			Self::UnsupportedMetadataVersion(v) => {
				write!(f, "Cannot convert v{v} metadata into Metadata type")
			},
			Self::TypeNameNotFound(name) => {
				write!(f, "Type name {name} is expected but not found in the type registry")
			},
			Self::InvalidTypePath(path) => write!(f, "Type has an invalid path {path}"),
			Self::StorageInfoError(e) => {
				write!(f, "Error decoding storage entry information: {e}")
			},
			Self::RuntimeInfoError(e) => {
				write!(f, "Error decoding Runtime API information: {e}")
			},
			Self::ViewFunctionInfoError(e) => {
				write!(f, "Error decoding View Function information: {e}")
			},
		}
	}
}

impl core::error::Error for TryFromError {}

impl From<frame_decode::storage::StorageInfoError<'static>> for TryFromError {
	fn from(e: frame_decode::storage::StorageInfoError<'static>) -> Self {
		Self::StorageInfoError(e)
	}
}

impl From<frame_decode::runtime_apis::RuntimeApiInfoError<'static>> for TryFromError {
	fn from(e: frame_decode::runtime_apis::RuntimeApiInfoError<'static>) -> Self {
		Self::RuntimeInfoError(e)
	}
}

impl From<frame_decode::view_functions::ViewFunctionInfoError<'static>> for TryFromError {
	fn from(e: frame_decode::view_functions::ViewFunctionInfoError<'static>) -> Self {
		Self::ViewFunctionInfoError(e)
	}
}

impl TryFrom<frame_metadata::RuntimeMetadataPrefixed> for crate::Metadata {
	type Error = TryFromError;

	fn try_from(value: frame_metadata::RuntimeMetadataPrefixed) -> Result<Self, Self::Error> {
		match value.1 {
			frame_metadata::RuntimeMetadata::V0(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(0))
			},
			frame_metadata::RuntimeMetadata::V1(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(1))
			},
			frame_metadata::RuntimeMetadata::V2(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(2))
			},
			frame_metadata::RuntimeMetadata::V3(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(3))
			},
			frame_metadata::RuntimeMetadata::V4(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(4))
			},
			frame_metadata::RuntimeMetadata::V5(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(5))
			},
			frame_metadata::RuntimeMetadata::V6(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(6))
			},
			frame_metadata::RuntimeMetadata::V7(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(7))
			},
			frame_metadata::RuntimeMetadata::V8(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(8))
			},
			frame_metadata::RuntimeMetadata::V9(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(9))
			},
			frame_metadata::RuntimeMetadata::V10(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(10))
			},
			frame_metadata::RuntimeMetadata::V11(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(11))
			},
			frame_metadata::RuntimeMetadata::V12(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(12))
			},
			frame_metadata::RuntimeMetadata::V13(_) => {
				Err(TryFromError::UnsupportedMetadataVersion(13))
			},
			frame_metadata::RuntimeMetadata::V14(m) => m.try_into(),
			frame_metadata::RuntimeMetadata::V15(m) => m.try_into(),
			frame_metadata::RuntimeMetadata::V16(m) => m.try_into(),
		}
	}
}
