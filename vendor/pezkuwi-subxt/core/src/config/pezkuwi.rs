// Copyright 2019-2026 Dijital Kurdistan Tech Institute
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

//! Pezkuwi specific configuration

use super::{Config, DefaultExtrinsicParams, DefaultExtrinsicParamsBuilder};

use crate::config::BizinikiwConfig;
pub use crate::utils::{AccountId32, MultiAddress, MultiSignature};
pub use primitive_types::{H256, U256};

/// Default set of commonly used types by Pezkuwi nodes.
// Note: The trait implementations exist just to make life easier,
// but shouldn't strictly be necessary since users can't instantiate this type.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum PezkuwiConfig {}

impl Config for PezkuwiConfig {
	type AccountId = <BizinikiwConfig as Config>::AccountId;
	type Signature = <BizinikiwConfig as Config>::Signature;
	type Hasher = <BizinikiwConfig as Config>::Hasher;
	type Header = <BizinikiwConfig as Config>::Header;
	type AssetId = <BizinikiwConfig as Config>::AssetId;

	// Address on Pezkuwi has no account index, whereas it's u32 on
	// the default bizinikiwi dev node.
	type Address = MultiAddress<Self::AccountId, ()>;

	// These are the same as the default bizinikiwi node, but redefined
	// because we need to pass the PezkuwiConfig trait as a param.
	type ExtrinsicParams = PezkuwiExtrinsicParams<Self>;
}

/// A struct representing the signed extra and additional parameters required
/// to construct a transaction for a pezkuwi node.
pub type PezkuwiExtrinsicParams<T> = DefaultExtrinsicParams<T>;

/// A builder which leads to [`PezkuwiExtrinsicParams`] being constructed.
/// This is what you provide to methods like `sign_and_submit()`.
pub type PezkuwiExtrinsicParamsBuilder<T> = DefaultExtrinsicParamsBuilder<T>;
