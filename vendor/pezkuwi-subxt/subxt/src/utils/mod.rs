// Copyright 2019-2026 Dijital Kurdistan Tech Institute
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

//! Miscellaneous utility helpers.

pub use pezkuwi_subxt_core::utils::{
	bits, strip_compact_prefix, to_hex, AccountId32, Encoded, Era, KeyedVec, MultiAddress,
	MultiSignature, PhantomDataSendSync, Static, UncheckedExtrinsic, WrapperKeepOpaque, Yes, H160,
	H256, H512,
};

pub use pezkuwi_subxt_rpcs::utils::url_is_secure;

crate::macros::cfg_jsonrpsee! {
	mod fetch_chain_spec;
	pub use fetch_chain_spec::{fetch_chainspec_from_rpc_node, FetchChainspecError};
}
