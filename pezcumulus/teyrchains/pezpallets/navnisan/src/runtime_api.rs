//! What a wallet asks in one call. A light client can instead read the same records from
//! storage with proofs (`Names[node]` per label, plus `Timestamp::Now` for a lease); this API
//! is the convenient path, not the only one.

use crate::{Content, Kind, Moment, NameHash, Owner, RegistrationId, Status};
use alloc::vec::Vec;
use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo)]
pub struct Resolution<AccountId, Office> {
	pub node: NameHash,
	pub account: Option<AccountId>,
	pub content: Option<Content<AccountId>>,
	pub owner: Owner<AccountId, Office>,
	pub kind: Kind,
	pub status: Status,
	/// The lease's own end; `None` for granted names and open-ended subnames.
	pub expiry: Option<Moment>,
	pub reg: RegistrationId,
	/// Granted, Parliament's, an office's, or under one of them. Only these may be shown as
	/// official; a bought name never is.
	pub official: bool,
}

#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, Debug, TypeInfo)]
pub enum Availability {
	#[codec(index = 0)]
	Free,
	#[codec(index = 1)]
	Taken,
	#[codec(index = 2)]
	Grace,
	#[codec(index = 3)]
	Locked,
	#[codec(index = 4)]
	Invalid,
	/// One or two letters: only granted.
	#[codec(index = 5)]
	GrantOnly,
}

pezsp_api::decl_runtime_apis! {
	pub trait NavnisanApi<AccountId, Office, Balance>
	where
		AccountId: Codec,
		Office: Codec,
		Balance: Codec,
	{
		/// `name` is the full dotted name ending in `.kurd`.
		fn resolve(name: Vec<u8>) -> Option<Resolution<AccountId, Office>>;
		fn text(name: Vec<u8>, key: Vec<u8>) -> Option<Vec<u8>>;
		/// The account's chosen name, checked forward.
		fn reverse(who: AccountId) -> Option<Vec<u8>>;
		/// Rent for a top-level label for `years`, if it may be bought.
		fn price(label: Vec<u8>, years: u32) -> Option<Balance>;
		fn availability(label: Vec<u8>) -> Availability;
	}
}
