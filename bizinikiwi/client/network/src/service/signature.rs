// This file is part of Bizinikiwi.
//
// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// If you read this, you are very thorough, congratulations.

//! Signature-related code

use std::fmt;

/// An error during signing of a message.
#[derive(Debug)]
pub struct SigningError {
	msg: String,
}

impl SigningError {
	/// Create a new [`SigningError`].
	pub fn new(msg: impl ToString) -> Self {
		Self { msg: msg.to_string() }
	}
}

impl fmt::Display for SigningError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Key signing error: {}", self.msg)
	}
}

impl std::error::Error for SigningError {}

/// An error while decoding a protobuf-encoded public key.
#[derive(Debug, thiserror::Error)]
#[error("Failed to decode public key: {0}")]
pub struct DecodingError(String);

/// Public key.
pub enum PublicKey {
	/// Litep2p public key.
	Litep2p(litep2p::crypto::PublicKey),
}

impl PublicKey {
	/// Protobuf-encode [`PublicKey`].
	pub fn encode_protobuf(&self) -> Vec<u8> {
		match self {
			Self::Litep2p(public) => public.to_protobuf_encoding(),
		}
	}

	/// Decode a protobuf-encoded ed25519 [`PublicKey`].
	pub fn try_decode_protobuf(bytes: &[u8]) -> Result<Self, DecodingError> {
		match litep2p::crypto::RemotePublicKey::from_protobuf_encoding(bytes)
			.map_err(|error| DecodingError(error.to_string()))?
		{
			litep2p::crypto::RemotePublicKey::Ed25519(public) => {
				Ok(Self::Litep2p(litep2p::crypto::PublicKey::Ed25519(public)))
			},
			#[allow(unreachable_patterns)]
			_ => Err(DecodingError("only ed25519 keys are supported".into())),
		}
	}

	/// Verify that `signature` was produced over `message` by the key behind this
	/// [`PublicKey`].
	pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
		match self {
			Self::Litep2p(litep2p::crypto::PublicKey::Ed25519(public)) => {
				public.verify(message, signature)
			},
		}
	}

	/// Get `PeerId` of the [`PublicKey`].
	pub fn to_peer_id(&self) -> pezsc_network_types::PeerId {
		match self {
			Self::Litep2p(public) => public.to_peer_id().into(),
		}
	}
}

/// Keypair.
pub enum Keypair {
	/// Litep2p keypair.
	Litep2p(litep2p::crypto::ed25519::Keypair),
}

impl Keypair {
	/// Generate ed25519 keypair.
	pub fn generate_ed25519() -> Self {
		Keypair::Litep2p(litep2p::crypto::ed25519::Keypair::generate())
	}

	/// Get [`Keypair`]'s public key.
	pub fn public(&self) -> PublicKey {
		match self {
			Keypair::Litep2p(keypair) => PublicKey::Litep2p(keypair.public().into()),
		}
	}
}

/// A result of signing a message with a network identity. Since `PeerId` is potentially a hash of a
/// `PublicKey`, you need to reveal the `PublicKey` next to the signature, so the verifier can check
/// if the signature was made by the entity that controls a given `PeerId`.
pub struct Signature {
	/// The public key derived from the network identity that signed the message.
	pub public_key: PublicKey,

	/// The actual signature made for the message signed.
	pub bytes: Vec<u8>,
}

impl Signature {
	/// Create new [`Signature`].
	pub fn new(public_key: PublicKey, bytes: Vec<u8>) -> Self {
		Self { public_key, bytes }
	}

	/// Create a signature for a message with a given network identity.
	pub fn sign_message(
		message: impl AsRef<[u8]>,
		keypair: &Keypair,
	) -> Result<Self, SigningError> {
		match keypair {
			Keypair::Litep2p(keypair) => {
				let public_key = keypair.public();
				let bytes = keypair.sign(message.as_ref());

				Ok(Signature { public_key: PublicKey::Litep2p(public_key.into()), bytes })
			},
		}
	}
}
