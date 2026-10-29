#![cfg_attr(not(feature = "std"), no_std)]

//! # Navnisan Pezpallet
//!
//! The `.kurd` name register: a name resolves to an account and to content (a site on IPFS), so
//! a wallet can open `dks.kurd` without asking a DNS server that a court can order to lie.
//!
//! ## Two kinds of name
//!
//! - **Leased** names are bought for one to five years at a time, paid in HEZ by length band,
//!   and the rent goes to the treasury. They can be renewed, transferred and sold, and when a
//!   lease runs out the name has a thirty-day grace period before anyone may buy it again.
//! - **Granted** names are given by Parliament, free, and never expire. They cannot be
//!   transferred or sold, and neither can any name underneath them. `gov.kurd` and the zone
//!   roots (`edu`, `org`, ...) are granted at genesis.
//!
//! Every name may carry subnames, managed by the parent's manager. A subname never outlives its
//! parent and dies with it: validity is checked on read, against the parent's registration
//! number, so nothing has to walk a subtree to delete it.
//!
//! ## Institutions change slowly
//!
//! An institution's site is the thing an attacker most wants to replace. A change to the
//! account, content or owner of an institutional name proposed by its own manager (an office
//! holder, or the institution's multisig) waits [`Config::InstitutionDelay`] before anyone can
//! enact it, and Parliament can cancel it in between. A news feed's daily anchor is the one
//! exception: it can only move the same feed forward, and it is posted by a key that can do
//! nothing else.
//!
//! ## What a light client reads
//!
//! Resolution is a handful of storage reads with no runtime code involved: `Names[node]` for
//! the name and each ancestor, plus `Timestamp::Now` when a lease is involved. That is why
//! there is no pluggable resolver, and why the storage layout here is part of the wallet's
//! interface: renaming the pallet or an item changes every proof key.
//!
//! ## Nothing here is writable from abroad
//!
//! Every origin is local. The People runtime's XCM call filter is an allowlist that does not
//! name this pallet, and a test in the runtime keeps it that way.

extern crate alloc;

pub use pezpallet::*;

pub mod genesis_set;
pub mod label;
pub mod migrations;
pub mod runtime_api;
pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use pezframe_support::{pezpallet_prelude::*, traits::fungible};
use pezsp_core::H256;
use scale_info::TypeInfo;

/// The node of a name: `blake2_256(parent_node ++ blake2_256(label))`, with `.kurd` itself as
/// the implicit root ([`KURD_NODE`]). ENS's namehash with blake2 in place of keccak.
pub type NameHash = H256;

/// Bumped for every new registration, so records keyed by an old registration -- subnames,
/// text records, a pending change -- stop being valid the moment their name is registered anew.
pub type RegistrationId = u64;

pub type BalanceOf<T> = <<T as Config>::Currency as fungible::Inspect<
	<T as pezframe_system::Config>::AccountId,
>>::Balance;

pub type CreditOf<T> =
	fungible::Credit<<T as pezframe_system::Config>::AccountId, <T as Config>::Currency>;

/// Milliseconds, as the Timestamp pallet keeps them. Leases are measured in time rather than
/// in blocks because the block time of these chains has already changed once.
pub type Moment = u64;

pub const MILLIS_PER_DAY: Moment = 24 * 60 * 60 * 1000;
pub const YEAR: Moment = 365 * MILLIS_PER_DAY;

fn blake2(data: &[u8]) -> [u8; 32] {
	use pezsp_runtime::traits::Hash;
	pezsp_runtime::traits::BlakeTwo256::hash(data).0
}

/// `node("kurd")`. Never stored: every top-level name's parent.
pub fn kurd_node() -> NameHash {
	H256(blake2(&[[0u8; 32], blake2(b"kurd")].concat()))
}

/// The node of `label` under `parent`.
pub fn child_node(parent: &NameHash, label: &[u8]) -> NameHash {
	H256(blake2(&[parent.0, blake2(label)].concat()))
}

/// The bytes the client commits to before `register`. `who` is inside, so a reveal seen in
/// the pool cannot be replayed by another account.
pub fn commitment<AccountId: Encode>(
	label: &[u8],
	owner: &AccountId,
	who: &AccountId,
	secret: &[u8; 32],
) -> H256 {
	H256(blake2(&(b"navnisan/commit", label, owner, who, secret).encode()))
}

/// Who manages a name.
#[derive(
	Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen,
)]
pub enum Owner<AccountId, Office> {
	/// An account: a person, or an institution's multisig.
	#[codec(index = 0)]
	Account(AccountId),
	/// Whoever holds a single-seat office at the time. A minister's name passes to the next
	/// minister without Parliament having to rewrite it.
	#[codec(index = 1)]
	Office(Office),
	/// Only Parliament: `gov.kurd` and the zone roots.
	#[codec(index = 2)]
	Governance,
}

#[derive(
	Clone,
	Copy,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum Kind {
	/// Bought, top-level, expires.
	#[codec(index = 0)]
	Leased,
	/// Given by Parliament, never expires, never transferable.
	#[codec(index = 1)]
	Granted,
	/// Under another name; lives at most as long as its parent.
	#[codec(index = 2)]
	Sub,
}

pub const MAX_CID: u32 = 64;
pub type Cid = BoundedVec<u8, ConstU32<MAX_CID>>;

/// What a name's site is.
#[derive(
	Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen,
)]
pub enum Content<AccountId> {
	/// A fixed site: an EIP-1577 contenthash (`0xe3` ipfs-ns + CIDv1).
	#[codec(index = 0)]
	Ipfs(Cid),
	/// A site that changes many times a day. The publisher key signs each release off chain
	/// (an IPNS-format record); `anchor` and `seq` are the newest release the chain has seen,
	/// a floor that stops a mirror from serving an older one. `anchorer` may move the anchor
	/// forward and do nothing else, so the key that does it every day can live on a server.
	#[codec(index = 1)]
	Feed { publisher: [u8; 32], anchorer: AccountId, anchor: Cid, seq: u64 },
}

/// A name.
#[derive(
	Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct NameRecord<AccountId, Office, Balance> {
	pub label: label::Label,
	pub parent: NameHash,
	/// The parent's `reg` when this subname was set; zero for top-level names.
	pub parent_reg: RegistrationId,
	pub reg: RegistrationId,
	pub owner: Owner<AccountId, Office>,
	pub kind: Kind,
	/// Set once, from the kind and the ancestors: a granted name and everything under it.
	pub locked_transfer: bool,
	/// `Some` for leases and for subnames given an end; `None` means "as long as the parent".
	pub expiry: Option<Moment>,
	pub account: Option<AccountId>,
	pub content: Option<Content<AccountId>>,
	/// A subname's deposit and who paid it.
	pub deposit: Option<(AccountId, Balance)>,
}

impl<A, O, B> NameRecord<A, O, B> {
	/// Granted names, Parliament's and offices' names, and everything under them.
	pub fn is_institutional(&self) -> bool {
		self.locked_transfer || !matches!(self.owner, Owner::Account(_))
	}
}

/// Where a name stands, computed on read and never stored.
#[derive(
	Clone,
	Copy,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum Status {
	#[codec(index = 0)]
	Active,
	/// The lease has ended; the site still resolves and only the last owner may renew.
	#[codec(index = 1)]
	Grace,
}

#[derive(
	Clone,
	Copy,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Default,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct PriceBands<Balance> {
	/// Yearly rent for a three-letter name.
	pub len3: Balance,
	pub len4: Balance,
	/// Five letters and longer.
	pub len5plus: Balance,
}

/// A change an institution's own manager proposed, waiting out [`Config::InstitutionDelay`].
#[derive(
	Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen,
)]
pub enum Change<AccountId, Office> {
	#[codec(index = 0)]
	Account(Option<AccountId>),
	#[codec(index = 1)]
	Content(Option<Content<AccountId>>),
	#[codec(index = 2)]
	Owner(Owner<AccountId, Office>),
}

impl<A, O> Change<A, O> {
	pub fn slot(&self) -> u8 {
		match self {
			Change::Account(_) => 0,
			Change::Content(_) => 1,
			Change::Owner(_) => 2,
		}
	}
}

#[derive(
	Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct Pending<AccountId, Office> {
	pub change: Change<AccountId, Office>,
	pub due: Moment,
	/// The registration the change was proposed against; a re-registered name drops it.
	pub reg: RegistrationId,
}

#[derive(
	Clone, Encode, Decode, DecodeWithMemTracking, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct Revocation {
	pub due: Moment,
	pub reg: RegistrationId,
	/// Hash of the court's written reasons.
	pub reason: H256,
}

#[cfg(feature = "runtime-benchmarks")]
pub trait BenchmarkHelper {
	fn set_time(ms: Moment);
}

/// Single-seat offices that can own a name. The runtime answers from the tiki register.
pub trait Offices<AccountId> {
	type Office: Parameter + MaxEncodedLen + Copy + pezsp_runtime::traits::MaybeSerializeDeserialize;
	/// Only offices one person holds at a time may own a name.
	fn is_single_seat(office: &Self::Office) -> bool;
	fn holder(office: &Self::Office) -> Option<AccountId>;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use alloc::vec::Vec;
	use pezframe_support::traits::{
		fungible::{Balanced, Inspect, Mutate, MutateHold},
		tokens::{Fortitude, Precision, Preservation},
		OnUnbalanced, UnixTime,
	};
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_runtime::traits::{Saturating, Zero};

	/// 1 from birth. A chain founded with this pallet writes it at genesis; a chain that gains
	/// the pallet in an upgrade starts at 0 and `migrations::v1::SeedNavnisan` writes the
	/// genesis set once.
	pub(crate) const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	pub type RecordOf<T> =
		NameRecord<<T as pezframe_system::Config>::AccountId, OfficeOf<T>, BalanceOf<T>>;
	pub type OfficeOf<T> =
		<<T as Config>::Offices as Offices<<T as pezframe_system::Config>::AccountId>>::Office;
	pub type OwnerOf<T> = Owner<<T as pezframe_system::Config>::AccountId, OfficeOf<T>>;
	pub type ContentOf<T> = Content<<T as pezframe_system::Config>::AccountId>;
	pub type ChangeOf<T> = Change<<T as pezframe_system::Config>::AccountId, OfficeOf<T>>;

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config<RuntimeEvent: From<Event<Self>>> {
		type Currency: Inspect<Self::AccountId>
			+ Mutate<Self::AccountId>
			+ Balanced<Self::AccountId>
			+ MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason>;

		type RuntimeHoldReason: From<HoldReason>;

		/// Where rent goes. The treasury, never `()`: rent is revenue, not a burn.
		type RentDestination: OnUnbalanced<CreditOf<Self>>;

		/// The chain's clock (the Timestamp pallet).
		type Time: UnixTime;

		type Offices: Offices<Self::AccountId>;

		/// Grants and revokes free names, locks labels, changes institutional names at once,
		/// cancels an institution's pending change. Parliament.
		type GrantOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Sets the yearly prices. Parliament.
		type PriceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Takes back a name bought in bad faith, after [`Config::DisputeDelay`]. The Diwan,
		/// two thirds.
		type DisputeOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		#[pezpallet::constant]
		type CommitDeposit: Get<BalanceOf<Self>>;
		#[pezpallet::constant]
		type SubnameDeposit: Get<BalanceOf<Self>>;
		#[pezpallet::constant]
		type TextDepositBase: Get<BalanceOf<Self>>;
		#[pezpallet::constant]
		type TextDepositPerByte: Get<BalanceOf<Self>>;

		/// A reveal must wait this long after its commitment: long enough that a collator
		/// who sees it cannot have committed to the same name in the blocks before.
		#[pezpallet::constant]
		type MinCommitmentAge: Get<Moment>;
		#[pezpallet::constant]
		type MaxCommitmentAge: Get<Moment>;
		#[pezpallet::constant]
		type GracePeriod: Get<Moment>;
		/// Most years paid in one call.
		#[pezpallet::constant]
		type MaxYears: Get<u32>;
		/// Furthest a lease may reach from now.
		#[pezpallet::constant]
		type MaxHorizon: Get<Moment>;
		#[pezpallet::constant]
		type InstitutionDelay: Get<Moment>;
		#[pezpallet::constant]
		type DisputeDelay: Get<Moment>;

		type WeightInfo: WeightInfo;

		/// Sets the chain's clock for benchmarks, which cannot wait out a commitment.
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: BenchmarkHelper;
	}

	#[pezpallet::composite_enum]
	pub enum HoldReason {
		#[codec(index = 0)]
		Commitment,
		#[codec(index = 1)]
		Subname,
		#[codec(index = 2)]
		Text,
	}

	#[pezpallet::storage]
	pub type Names<T: Config> = StorageMap<_, Identity, NameHash, RecordOf<T>, OptionQuery>;

	/// Free-form records of a registration (`email`, `url`, ...), keyed by `reg` so that a
	/// name registered anew starts with none.
	#[pezpallet::storage]
	pub type TextRecords<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		RegistrationId,
		Blake2_128Concat,
		label::TextKey,
		(label::TextValue, T::AccountId, BalanceOf<T>),
		OptionQuery,
	>;

	#[pezpallet::storage]
	pub type TextCount<T: Config> = StorageMap<_, Twox64Concat, RegistrationId, u8, ValueQuery>;

	/// An account's chosen name. Checked forward on every read: if the name no longer
	/// resolves to the account, the entry is stale and anyone may remove it.
	#[pezpallet::storage]
	pub type Primary<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, NameHash, OptionQuery>;

	#[pezpallet::storage]
	pub type Commitments<T: Config> =
		StorageMap<_, Identity, H256, (T::AccountId, Moment, BalanceOf<T>), OptionQuery>;

	/// Top-level labels nobody may buy; Parliament grants them. The value is a category for
	/// explorers (state, institution, nation, city, title, safety).
	#[pezpallet::storage]
	pub type Locked<T: Config> = StorageMap<_, Identity, NameHash, u8, OptionQuery>;

	#[pezpallet::storage]
	pub type Prices<T: Config> = StorageValue<_, PriceBands<BalanceOf<T>>, ValueQuery>;

	#[pezpallet::storage]
	pub type NextRegistration<T: Config> = StorageValue<_, RegistrationId, ValueQuery>;

	#[pezpallet::storage]
	pub type PendingChanges<T: Config> = StorageDoubleMap<
		_,
		Identity,
		NameHash,
		Twox64Concat,
		u8,
		Pending<T::AccountId, OfficeOf<T>>,
		OptionQuery,
	>;

	#[pezpallet::storage]
	pub type PendingRevocations<T: Config> =
		StorageMap<_, Identity, NameHash, Revocation, OptionQuery>;

	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// Top-level names owned by Parliament: `gov` and the zone roots.
		pub zones: Vec<Vec<u8>>,
		/// Names under `gov.kurd`; `Some(office)` makes that office the manager.
		pub gov_subnames: Vec<(Vec<u8>, Option<OfficeOf<T>>)>,
		/// Top-level labels nobody may buy, with their category.
		pub locked: Vec<(Vec<u8>, u8)>,
		pub prices: PriceBands<BalanceOf<T>>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			// An empty config (a test runtime's default) writes nothing; a real chain spec
			// always carries prices, and `seed` refuses zero ones.
			if self.zones.is_empty()
				&& self.gov_subnames.is_empty()
				&& self.locked.is_empty()
				&& self.prices == PriceBands::default()
			{
				return;
			}
			Pezpallet::<T>::seed(&self.zones, &self.gov_subnames, &self.locked, self.prices)
				.expect("the navnisan genesis set is valid");
		}
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Committed {
			who: T::AccountId,
			commitment: H256,
		},
		Registered {
			node: NameHash,
			label: label::Label,
			owner: T::AccountId,
			expiry: Moment,
			reg: RegistrationId,
			paid: BalanceOf<T>,
		},
		Renewed {
			node: NameHash,
			expiry: Moment,
			paid: BalanceOf<T>,
			by: T::AccountId,
		},
		Transferred {
			node: NameHash,
			from: T::AccountId,
			to: T::AccountId,
		},
		SubnameSet {
			node: NameHash,
			parent: NameHash,
			label: label::Label,
			owner: OwnerOf<T>,
			expiry: Option<Moment>,
			reg: RegistrationId,
		},
		SubnameRemoved {
			node: NameHash,
		},
		Granted {
			node: NameHash,
			parent: NameHash,
			label: label::Label,
			owner: OwnerOf<T>,
		},
		Revoked {
			node: NameHash,
		},
		AccountSet {
			node: NameHash,
			account: Option<T::AccountId>,
		},
		ContentSet {
			node: NameHash,
			content: Option<ContentOf<T>>,
		},
		OwnerSet {
			node: NameHash,
			owner: OwnerOf<T>,
		},
		AnchorMoved {
			node: NameHash,
			anchor: Cid,
			seq: u64,
		},
		/// The value is in storage; the event carries its hash to stay small.
		TextSet {
			node: NameHash,
			key: label::TextKey,
			value_hash: Option<H256>,
		},
		PrimarySet {
			who: T::AccountId,
			node: NameHash,
		},
		PrimaryCleared {
			who: T::AccountId,
		},
		ChangeProposed {
			node: NameHash,
			change: ChangeOf<T>,
			due: Moment,
		},
		ChangeCancelled {
			node: NameHash,
			slot: u8,
		},
		RevocationScheduled {
			node: NameHash,
			due: Moment,
			reason: H256,
		},
		RevocationCancelled {
			node: NameHash,
		},
		LabelLocked {
			node: NameHash,
			label: label::Label,
			category: u8,
		},
		LabelUnlocked {
			node: NameHash,
		},
		PricesSet {
			prices: PriceBands<BalanceOf<T>>,
		},
		Reaped {
			node: NameHash,
			reg: RegistrationId,
		},
		CommitmentRemoved {
			commitment: H256,
		},
	}

	#[pezpallet::error]
	pub enum Error<T> {
		BadLabel,
		/// Bought names are three to sixty-three letters; shorter ones are only granted.
		TooShortToBuy,
		LabelLocked,
		NameTaken,
		UnknownName,
		NotManager,
		NotTransferable,
		/// The name is in its grace period or has lapsed.
		NotActive,
		NoCommitment,
		CommitmentExists,
		CommitmentTooNew,
		CommitmentTooOld,
		CommitmentNotExpired,
		BadYears,
		BeyondHorizon,
		/// Renewing in the grace period is the last owner's right.
		GraceOwnerOnly,
		NotLeased,
		TooDeep,
		/// Only names in a granted tree are granted or revoked.
		NotInstitutional,
		/// An institutional name changes through a pending change, not directly.
		Institutional,
		BadOffice,
		BadContent,
		NotAFeed,
		AnchorNotNewer,
		NotAnchorer,
		TooManyTexts,
		NoText,
		NothingPending,
		NotDue,
		NotStale,
		BadPrices,
		AlreadyScheduled,
		NotPrimary,
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			Self::do_try_state()
		}
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Commit to buying a name without revealing it. Holds [`Config::CommitDeposit`].
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::commit())]
		pub fn commit(origin: OriginFor<T>, commitment: H256) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(!Commitments::<T>::contains_key(commitment), Error::<T>::CommitmentExists);
			let deposit = T::CommitDeposit::get();
			T::Currency::hold(&HoldReason::Commitment.into(), &who, deposit)?;
			Commitments::<T>::insert(commitment, (who.clone(), Self::now(), deposit));
			Self::deposit_event(Event::Committed { who, commitment });
			Ok(())
		}

		/// Reveal and buy a top-level name for `years` (one to [`Config::MaxYears`]).
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::register(label::MAX_TEXTS))]
		pub fn register(
			origin: OriginFor<T>,
			label: Vec<u8>,
			owner: T::AccountId,
			secret: [u8; 32],
			years: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let label =
				label::validate(&label, label::Use::Buy).map_err(|e| Self::label_error(e))?;
			let c = commitment(&label, &owner, &who, &secret);
			let (committer, at, deposit) =
				Commitments::<T>::get(c).ok_or(Error::<T>::NoCommitment)?;
			ensure!(committer == who, Error::<T>::NoCommitment);
			let now = Self::now();
			ensure!(
				at.saturating_add(T::MinCommitmentAge::get()) <= now,
				Error::<T>::CommitmentTooNew
			);
			ensure!(
				now <= at.saturating_add(T::MaxCommitmentAge::get()),
				Error::<T>::CommitmentTooOld
			);

			let node = child_node(&kurd_node(), &label);
			ensure!(!Locked::<T>::contains_key(node), Error::<T>::LabelLocked);
			ensure!(!PendingRevocations::<T>::contains_key(node), Error::<T>::NameTaken);
			if let Some(old) = Names::<T>::get(node) {
				ensure!(Self::status(&old, now).is_none(), Error::<T>::NameTaken);
				Self::clear_registration(&old);
			}
			let expiry = Self::lease_end(now, now, years)?;
			let paid = Self::charge_rent(&who, label.len(), years)?;

			Commitments::<T>::remove(c);
			T::Currency::release(
				&HoldReason::Commitment.into(),
				&who,
				deposit,
				Precision::BestEffort,
			)?;

			let reg = Self::next_reg();
			Names::<T>::insert(
				node,
				NameRecord {
					label: label.clone(),
					parent: kurd_node(),
					parent_reg: 0,
					reg,
					owner: Owner::Account(owner.clone()),
					kind: Kind::Leased,
					locked_transfer: false,
					expiry: Some(expiry),
					account: Some(owner.clone()),
					content: None,
					deposit: None,
				},
			);
			Self::deposit_event(Event::Registered { node, label, owner, expiry, reg, paid });
			Ok(())
		}

		/// Extend a lease. Anyone may pay for an active name; in the grace period only its
		/// last owner may.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::renew())]
		pub fn renew(origin: OriginFor<T>, node: NameHash, years: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut rec = Names::<T>::get(node).ok_or(Error::<T>::UnknownName)?;
			ensure!(rec.kind == Kind::Leased, Error::<T>::NotLeased);
			let now = Self::now();
			match Self::status(&rec, now) {
				None => return Err(Error::<T>::NotActive.into()),
				Some(Status::Grace) => {
					ensure!(rec.owner == Owner::Account(who.clone()), Error::<T>::GraceOwnerOnly)
				},
				Some(Status::Active) => {},
			}
			let from = rec.expiry.unwrap_or(now).max(now);
			let expiry = Self::lease_end(now, from, years)?;
			let paid = Self::charge_rent(&who, rec.label.len(), years)?;
			rec.expiry = Some(expiry);
			Names::<T>::insert(node, rec);
			Self::deposit_event(Event::Renewed { node, expiry, paid, by: who });
			Ok(())
		}

		/// Hand a leased name, or a transferable subname, to another account. The name's
		/// account record moves with it, so payments do not keep going to the seller.
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(T::WeightInfo::transfer(label::MAX_DEPTH))]
		pub fn transfer(origin: OriginFor<T>, node: NameHash, to: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut rec = Self::managed_active(&who, node)?;
			ensure!(
				!rec.locked_transfer && matches!(rec.kind, Kind::Leased | Kind::Sub),
				Error::<T>::NotTransferable
			);
			rec.owner = Owner::Account(to.clone());
			rec.account = Some(to.clone());
			Names::<T>::insert(node, rec);
			Self::deposit_event(Event::Transferred { node, from: who, to });
			Ok(())
		}

		/// Point the name at an account. An institutional name waits out the delay.
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(T::WeightInfo::set_account(label::MAX_DEPTH))]
		pub fn set_account(
			origin: OriginFor<T>,
			node: NameHash,
			account: Option<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::change_by_manager(&who, node, Change::Account(account))
		}

		/// Point the name at a site. An institutional name waits out the delay.
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(T::WeightInfo::set_content(label::MAX_DEPTH))]
		pub fn set_content(
			origin: OriginFor<T>,
			node: NameHash,
			content: Option<ContentOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			if let Some(c) = &content {
				ensure!(Self::content_is_valid(c), Error::<T>::BadContent);
			}
			Self::change_by_manager(&who, node, Change::Content(content))
		}

		/// Move a feed's anchor forward. Only the feed's anchorer, only to a higher `seq`.
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(T::WeightInfo::anchor(label::MAX_DEPTH))]
		pub fn anchor(
			origin: OriginFor<T>,
			node: NameHash,
			anchor: Cid,
			seq: u64,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(label::is_contenthash(&anchor), Error::<T>::BadContent);
			let mut rec = Names::<T>::get(node).ok_or(Error::<T>::UnknownName)?;
			ensure!(Self::effective_status(&rec, Self::now()).is_some(), Error::<T>::NotActive);
			let Some(Content::Feed { anchorer, seq: old, anchor: a, .. }) = rec.content.as_mut()
			else {
				return Err(Error::<T>::NotAFeed.into());
			};
			ensure!(*anchorer == who, Error::<T>::NotAnchorer);
			ensure!(seq > *old, Error::<T>::AnchorNotNewer);
			*old = seq;
			*a = anchor.clone();
			Names::<T>::insert(node, rec);
			Self::deposit_event(Event::AnchorMoved { node, anchor, seq });
			Ok(())
		}

		/// Set or clear a text record. Holds a deposit by size from the caller.
		#[pezpallet::call_index(7)]
		#[pezpallet::weight(T::WeightInfo::set_text(label::MAX_DEPTH))]
		pub fn set_text(
			origin: OriginFor<T>,
			node: NameHash,
			key: label::TextKey,
			value: Option<label::TextValue>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let rec = Self::managed_active(&who, node)?;
			ensure!(label::is_text_key(&key), Error::<T>::BadLabel);
			let old = TextRecords::<T>::take(rec.reg, &key);
			if let Some((_, payer, held)) = &old {
				T::Currency::release(
					&HoldReason::Text.into(),
					payer,
					*held,
					Precision::BestEffort,
				)?;
			}
			let value_hash = match value {
				Some(v) => {
					if old.is_none() {
						let n = TextCount::<T>::get(rec.reg);
						ensure!((n as u32) < label::MAX_TEXTS, Error::<T>::TooManyTexts);
						TextCount::<T>::insert(rec.reg, n + 1);
					}
					let bytes = (key.len() + v.len()) as u32;
					let deposit = T::TextDepositBase::get()
						.saturating_add(T::TextDepositPerByte::get().saturating_mul(bytes.into()));
					T::Currency::hold(&HoldReason::Text.into(), &who, deposit)?;
					let h = H256(blake2(&v));
					TextRecords::<T>::insert(rec.reg, &key, (v, who.clone(), deposit));
					Some(h)
				},
				None => {
					ensure!(old.is_some(), Error::<T>::NoText);
					TextCount::<T>::mutate(rec.reg, |n| *n = n.saturating_sub(1));
					None
				},
			};
			Self::deposit_event(Event::TextSet { node, key, value_hash });
			Ok(())
		}

		/// Create or replace a subname. The parent's manager decides; an existing subname is
		/// replaced outright, which is how a parent takes one back.
		#[pezpallet::call_index(8)]
		#[pezpallet::weight(T::WeightInfo::set_subname(label::MAX_DEPTH, label::MAX_TEXTS))]
		pub fn set_subname(
			origin: OriginFor<T>,
			parent: NameHash,
			label: Vec<u8>,
			owner: T::AccountId,
			expiry: Option<Moment>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let parent_rec = Self::managed_active(&who, parent)?;
			// Under an institution only Parliament adds or replaces names (`grant`).
			ensure!(!parent_rec.is_institutional(), Error::<T>::Institutional);
			Self::put_subname(parent, &parent_rec, &label, Owner::Account(owner), expiry, Some(who))
		}

		/// Remove a subname: its parent's manager or its own.
		#[pezpallet::call_index(9)]
		#[pezpallet::weight(T::WeightInfo::remove_subname(label::MAX_TEXTS))]
		pub fn remove_subname(origin: OriginFor<T>, node: NameHash) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let rec = Names::<T>::get(node).ok_or(Error::<T>::UnknownName)?;
			ensure!(rec.kind == Kind::Sub, Error::<T>::UnknownName);
			// Taking an institution's site down is Parliament's `revoke`, not one key's.
			ensure!(!rec.is_institutional(), Error::<T>::Institutional);
			let own = Self::manager(&rec.owner) == Some(who.clone());
			let parents = Names::<T>::get(rec.parent)
				.and_then(|p| Self::manager(&p.owner))
				.is_some_and(|m| m == who);
			ensure!(own || parents, Error::<T>::NotManager);
			Self::delete(node, &rec);
			Self::deposit_event(Event::SubnameRemoved { node });
			Ok(())
		}

		/// Show `node` as the caller's name. It must resolve to the caller.
		#[pezpallet::call_index(10)]
		#[pezpallet::weight(T::WeightInfo::set_primary(label::MAX_DEPTH))]
		pub fn set_primary(origin: OriginFor<T>, node: NameHash) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::resolves_to(node, &who), Error::<T>::NotPrimary);
			Primary::<T>::insert(&who, node);
			Self::deposit_event(Event::PrimarySet { who, node });
			Ok(())
		}

		#[pezpallet::call_index(11)]
		#[pezpallet::weight(T::WeightInfo::clear_primary())]
		pub fn clear_primary(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Primary::<T>::take(&who).is_some(), Error::<T>::NotPrimary);
			Self::deposit_event(Event::PrimaryCleared { who });
			Ok(())
		}

		/// Give a free name: at the top level (any length, locked labels included) or under a
		/// name in a granted tree. Never expires, never transferable.
		#[pezpallet::call_index(12)]
		#[pezpallet::weight(T::WeightInfo::grant(label::MAX_DEPTH))]
		pub fn grant(
			origin: OriginFor<T>,
			parent: Option<NameHash>,
			label: Vec<u8>,
			owner: OwnerOf<T>,
		) -> DispatchResult {
			T::GrantOrigin::ensure_origin(origin)?;
			Self::do_grant(parent, &label, owner)
		}

		/// Take back a name in a granted tree. Its subnames die with it.
		#[pezpallet::call_index(13)]
		#[pezpallet::weight(T::WeightInfo::revoke(label::MAX_TEXTS))]
		pub fn revoke(origin: OriginFor<T>, node: NameHash) -> DispatchResult {
			T::GrantOrigin::ensure_origin(origin)?;
			let rec = Names::<T>::get(node).ok_or(Error::<T>::UnknownName)?;
			ensure!(rec.is_institutional(), Error::<T>::NotInstitutional);
			Self::delete(node, &rec);
			Self::deposit_event(Event::Revoked { node });
			Ok(())
		}

		/// Parliament changes an institutional name at once.
		#[pezpallet::call_index(14)]
		#[pezpallet::weight(T::WeightInfo::force_change())]
		pub fn force_change(
			origin: OriginFor<T>,
			node: NameHash,
			change: ChangeOf<T>,
		) -> DispatchResult {
			T::GrantOrigin::ensure_origin(origin)?;
			let mut rec = Names::<T>::get(node).ok_or(Error::<T>::UnknownName)?;
			ensure!(rec.is_institutional(), Error::<T>::NotInstitutional);
			Self::check_change(&change)?;
			PendingChanges::<T>::remove(node, change.slot());
			Self::apply(node, &mut rec, change);
			Names::<T>::insert(node, rec);
			Ok(())
		}

		/// Parliament stops an institution's pending change.
		#[pezpallet::call_index(15)]
		#[pezpallet::weight(T::WeightInfo::cancel_change())]
		pub fn cancel_change(origin: OriginFor<T>, node: NameHash, slot: u8) -> DispatchResult {
			T::GrantOrigin::ensure_origin(origin)?;
			ensure!(PendingChanges::<T>::take(node, slot).is_some(), Error::<T>::NothingPending);
			Self::deposit_event(Event::ChangeCancelled { node, slot });
			Ok(())
		}

		/// Bring a pending change into force once its delay has passed. Anyone may.
		#[pezpallet::call_index(16)]
		#[pezpallet::weight(T::WeightInfo::enact_change())]
		pub fn enact_change(origin: OriginFor<T>, node: NameHash, slot: u8) -> DispatchResult {
			ensure_signed(origin)?;
			let p = PendingChanges::<T>::get(node, slot).ok_or(Error::<T>::NothingPending)?;
			ensure!(p.due <= Self::now(), Error::<T>::NotDue);
			PendingChanges::<T>::remove(node, slot);
			let mut rec = Names::<T>::get(node).ok_or(Error::<T>::UnknownName)?;
			ensure!(rec.reg == p.reg, Error::<T>::NothingPending);
			Self::apply(node, &mut rec, p.change);
			Names::<T>::insert(node, rec);
			Ok(())
		}

		/// The court takes back a name bought in bad faith, after [`Config::DisputeDelay`].
		#[pezpallet::call_index(17)]
		#[pezpallet::weight(T::WeightInfo::schedule_revocation())]
		pub fn schedule_revocation(
			origin: OriginFor<T>,
			node: NameHash,
			reason: H256,
		) -> DispatchResult {
			T::DisputeOrigin::ensure_origin(origin)?;
			let rec = Names::<T>::get(node).ok_or(Error::<T>::UnknownName)?;
			ensure!(rec.kind == Kind::Leased, Error::<T>::NotLeased);
			ensure!(!PendingRevocations::<T>::contains_key(node), Error::<T>::AlreadyScheduled);
			let due = Self::now().saturating_add(T::DisputeDelay::get());
			PendingRevocations::<T>::insert(node, Revocation { due, reg: rec.reg, reason });
			Self::deposit_event(Event::RevocationScheduled { node, due, reason });
			Ok(())
		}

		#[pezpallet::call_index(18)]
		#[pezpallet::weight(T::WeightInfo::cancel_revocation())]
		pub fn cancel_revocation(origin: OriginFor<T>, node: NameHash) -> DispatchResult {
			T::DisputeOrigin::ensure_origin(origin)?;
			ensure!(PendingRevocations::<T>::take(node).is_some(), Error::<T>::NothingPending);
			Self::deposit_event(Event::RevocationCancelled { node });
			Ok(())
		}

		/// Carry out a scheduled revocation once due. Anyone may. No rent is returned.
		#[pezpallet::call_index(19)]
		#[pezpallet::weight(T::WeightInfo::enact_revocation(label::MAX_TEXTS))]
		pub fn enact_revocation(origin: OriginFor<T>, node: NameHash) -> DispatchResult {
			ensure_signed(origin)?;
			let r = PendingRevocations::<T>::get(node).ok_or(Error::<T>::NothingPending)?;
			ensure!(r.due <= Self::now(), Error::<T>::NotDue);
			PendingRevocations::<T>::remove(node);
			if let Some(rec) = Names::<T>::get(node) {
				if rec.reg == r.reg {
					Self::delete(node, &rec);
					Self::deposit_event(Event::Revoked { node });
				}
			}
			Ok(())
		}

		#[pezpallet::call_index(20)]
		#[pezpallet::weight(T::WeightInfo::lock())]
		pub fn lock(origin: OriginFor<T>, label: Vec<u8>, category: u8) -> DispatchResult {
			T::GrantOrigin::ensure_origin(origin)?;
			let label =
				label::validate(&label, label::Use::Grant).map_err(|e| Self::label_error(e))?;
			let node = child_node(&kurd_node(), &label);
			Locked::<T>::insert(node, category);
			Self::deposit_event(Event::LabelLocked { node, label, category });
			Ok(())
		}

		#[pezpallet::call_index(21)]
		#[pezpallet::weight(T::WeightInfo::unlock())]
		pub fn unlock(origin: OriginFor<T>, node: NameHash) -> DispatchResult {
			T::GrantOrigin::ensure_origin(origin)?;
			ensure!(Locked::<T>::take(node).is_some(), Error::<T>::UnknownName);
			Self::deposit_event(Event::LabelUnlocked { node });
			Ok(())
		}

		#[pezpallet::call_index(22)]
		#[pezpallet::weight(T::WeightInfo::set_prices())]
		pub fn set_prices(
			origin: OriginFor<T>,
			prices: PriceBands<BalanceOf<T>>,
		) -> DispatchResult {
			T::PriceOrigin::ensure_origin(origin)?;
			ensure!(Self::prices_are_valid(&prices), Error::<T>::BadPrices);
			Prices::<T>::put(prices);
			Self::deposit_event(Event::PricesSet { prices });
			Ok(())
		}

		/// Remove a lapsed lease or a subname whose parent is gone. Anyone may; deposits go
		/// back to whoever paid them.
		#[pezpallet::call_index(23)]
		#[pezpallet::weight(T::WeightInfo::reap(label::MAX_DEPTH, label::MAX_TEXTS))]
		pub fn reap(origin: OriginFor<T>, node: NameHash) -> DispatchResult {
			ensure_signed(origin)?;
			let rec = Names::<T>::get(node).ok_or(Error::<T>::UnknownName)?;
			ensure!(Self::effective_status(&rec, Self::now()).is_none(), Error::<T>::NotStale);
			Self::delete(node, &rec);
			Self::deposit_event(Event::Reaped { node, reg: rec.reg });
			Ok(())
		}

		/// Remove an account's chosen name once it no longer resolves to the account.
		#[pezpallet::call_index(24)]
		#[pezpallet::weight(T::WeightInfo::reap_primary(label::MAX_DEPTH))]
		pub fn reap_primary(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			ensure_signed(origin)?;
			let node = Primary::<T>::get(&who).ok_or(Error::<T>::NotPrimary)?;
			ensure!(!Self::resolves_to(node, &who), Error::<T>::NotStale);
			Primary::<T>::remove(&who);
			Self::deposit_event(Event::PrimaryCleared { who });
			Ok(())
		}

		/// Release an expired commitment's deposit to its committer. Anyone may.
		#[pezpallet::call_index(25)]
		#[pezpallet::weight(T::WeightInfo::remove_commitment())]
		pub fn remove_commitment(origin: OriginFor<T>, commitment: H256) -> DispatchResult {
			ensure_signed(origin)?;
			let (who, at, deposit) =
				Commitments::<T>::get(commitment).ok_or(Error::<T>::NoCommitment)?;
			ensure!(
				Self::now() > at.saturating_add(T::MaxCommitmentAge::get()),
				Error::<T>::CommitmentNotExpired
			);
			Commitments::<T>::remove(commitment);
			T::Currency::release(
				&HoldReason::Commitment.into(),
				&who,
				deposit,
				Precision::BestEffort,
			)?;
			Self::deposit_event(Event::CommitmentRemoved { commitment });
			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		pub fn now() -> Moment {
			T::Time::now().as_millis().try_into().unwrap_or(Moment::MAX)
		}

		fn label_error(e: label::LabelError) -> DispatchError {
			match e {
				label::LabelError::TooShortToBuy => Error::<T>::TooShortToBuy.into(),
				_ => Error::<T>::BadLabel.into(),
			}
		}

		fn next_reg() -> RegistrationId {
			NextRegistration::<T>::mutate(|n| {
				*n = n.saturating_add(1);
				*n
			})
		}

		fn lease_end(now: Moment, from: Moment, years: u32) -> Result<Moment, DispatchError> {
			ensure!(years >= 1 && years <= T::MaxYears::get(), Error::<T>::BadYears);
			let end = from.saturating_add(YEAR.saturating_mul(years as u64));
			ensure!(end <= now.saturating_add(T::MaxHorizon::get()), Error::<T>::BeyondHorizon);
			Ok(end)
		}

		pub fn price(len: usize, years: u32) -> BalanceOf<T> {
			let p = Prices::<T>::get();
			let band = match len {
				0..=2 => return Zero::zero(),
				3 => p.len3,
				4 => p.len4,
				_ => p.len5plus,
			};
			band.saturating_mul(years.into())
		}

		fn charge_rent(
			who: &T::AccountId,
			len: usize,
			years: u32,
		) -> Result<BalanceOf<T>, DispatchError> {
			let fee = Self::price(len, years);
			if !fee.is_zero() {
				let credit = T::Currency::withdraw(
					who,
					fee,
					Precision::Exact,
					Preservation::Preserve,
					Fortitude::Polite,
				)?;
				T::RentDestination::on_unbalanced(credit);
			}
			Ok(fee)
		}

		pub fn prices_are_valid(p: &PriceBands<BalanceOf<T>>) -> bool {
			!p.len5plus.is_zero() && p.len3 >= p.len4 && p.len4 >= p.len5plus
		}

		pub fn manager(owner: &OwnerOf<T>) -> Option<T::AccountId> {
			match owner {
				Owner::Account(a) => Some(a.clone()),
				Owner::Office(o) => T::Offices::holder(o),
				Owner::Governance => None,
			}
		}

		/// A record's own standing, ignoring its ancestors.
		pub fn status(rec: &RecordOf<T>, now: Moment) -> Option<Status> {
			match rec.expiry {
				None => Some(Status::Active),
				Some(end) if now < end => Some(Status::Active),
				Some(end)
					if rec.kind == Kind::Leased
						&& now < end.saturating_add(T::GracePeriod::get()) =>
				{
					Some(Status::Grace)
				},
				Some(_) => None,
			}
		}

		/// A record's standing with its ancestors: a subname is valid only while every
		/// ancestor is, under the registration it was set against.
		pub fn effective_status(rec: &RecordOf<T>, now: Moment) -> Option<Status> {
			let mut status = Self::status(rec, now)?;
			let mut cur = rec.clone();
			for _ in 0..label::MAX_DEPTH {
				if cur.kind != Kind::Sub {
					return Some(status);
				}
				let parent = Names::<T>::get(cur.parent)?;
				if parent.reg != cur.parent_reg {
					return None;
				}
				if Self::status(&parent, now)? == Status::Grace {
					status = Status::Grace;
				}
				cur = parent;
			}
			(cur.kind != Kind::Sub).then_some(status)
		}

		fn managed_active(
			who: &T::AccountId,
			node: NameHash,
		) -> Result<RecordOf<T>, DispatchError> {
			let rec = Names::<T>::get(node).ok_or(Error::<T>::UnknownName)?;
			ensure!(Self::manager(&rec.owner).as_ref() == Some(who), Error::<T>::NotManager);
			ensure!(
				Self::effective_status(&rec, Self::now()) == Some(Status::Active),
				Error::<T>::NotActive
			);
			Ok(rec)
		}

		fn check_change(change: &ChangeOf<T>) -> DispatchResult {
			match change {
				Change::Content(Some(c)) => {
					ensure!(Self::content_is_valid(c), Error::<T>::BadContent)
				},
				Change::Owner(Owner::Office(o)) => {
					ensure!(T::Offices::is_single_seat(o), Error::<T>::BadOffice)
				},
				_ => {},
			}
			Ok(())
		}

		fn change_by_manager(
			who: &T::AccountId,
			node: NameHash,
			change: ChangeOf<T>,
		) -> DispatchResult {
			let mut rec = Self::managed_active(who, node)?;
			Self::check_change(&change)?;
			if rec.is_institutional() {
				let due = Self::now().saturating_add(T::InstitutionDelay::get());
				PendingChanges::<T>::insert(
					node,
					change.slot(),
					Pending { change: change.clone(), due, reg: rec.reg },
				);
				Self::deposit_event(Event::ChangeProposed { node, change, due });
			} else {
				Self::apply(node, &mut rec, change);
				Names::<T>::insert(node, rec);
			}
			Ok(())
		}

		fn apply(node: NameHash, rec: &mut RecordOf<T>, change: ChangeOf<T>) {
			match change {
				Change::Account(a) => {
					rec.account = a.clone();
					Self::deposit_event(Event::AccountSet { node, account: a });
				},
				Change::Content(c) => {
					rec.content = c.clone();
					Self::deposit_event(Event::ContentSet { node, content: c });
				},
				Change::Owner(o) => {
					rec.owner = o.clone();
					Self::deposit_event(Event::OwnerSet { node, owner: o });
				},
			}
		}

		pub fn content_is_valid(c: &ContentOf<T>) -> bool {
			match c {
				Content::Ipfs(cid) => label::is_contenthash(cid),
				Content::Feed { anchor, .. } => label::is_contenthash(anchor),
			}
		}

		fn put_subname(
			parent: NameHash,
			parent_rec: &RecordOf<T>,
			raw: &[u8],
			owner: OwnerOf<T>,
			expiry: Option<Moment>,
			payer: Option<T::AccountId>,
		) -> DispatchResult {
			let label = label::validate(raw, label::Use::Sub).map_err(|e| Self::label_error(e))?;
			ensure!(Self::depth_of(parent_rec)? < label::MAX_DEPTH, Error::<T>::TooDeep);
			let node = child_node(&parent, &label);
			if let Some(old) = Names::<T>::get(node) {
				Self::clear_registration(&old);
				let _ = PendingChanges::<T>::clear_prefix(node, 3, None);
			}
			let deposit = match payer {
				Some(p) => {
					let d = T::SubnameDeposit::get();
					T::Currency::hold(&HoldReason::Subname.into(), &p, d)?;
					Some((p, d))
				},
				None => None,
			};
			let reg = Self::next_reg();
			let locked_transfer = parent_rec.locked_transfer
				|| parent_rec.kind == Kind::Granted
				|| parent_rec.owner == Owner::Governance;
			let account = Self::manager(&owner);
			Names::<T>::insert(
				node,
				NameRecord {
					label: label.clone(),
					parent,
					parent_reg: parent_rec.reg,
					reg,
					owner: owner.clone(),
					kind: Kind::Sub,
					locked_transfer,
					expiry,
					account,
					content: None,
					deposit,
				},
			);
			Self::deposit_event(Event::SubnameSet { node, parent, label, owner, expiry, reg });
			Ok(())
		}

		/// Names above `rec`, `.kurd` excluded: a top-level name is at depth one.
		fn depth_of(rec: &RecordOf<T>) -> Result<u32, DispatchError> {
			let mut depth = 1;
			let mut cur = rec.clone();
			while cur.kind == Kind::Sub {
				cur = Names::<T>::get(cur.parent).ok_or(Error::<T>::UnknownName)?;
				depth += 1;
				ensure!(depth <= label::MAX_DEPTH, Error::<T>::TooDeep);
			}
			Ok(depth)
		}

		pub fn do_grant(parent: Option<NameHash>, raw: &[u8], owner: OwnerOf<T>) -> DispatchResult {
			if let Owner::Office(o) = &owner {
				ensure!(T::Offices::is_single_seat(o), Error::<T>::BadOffice);
			}
			match parent {
				None => {
					let label = label::validate(raw, label::Use::Grant)
						.map_err(|e| Self::label_error(e))?;
					let node = child_node(&kurd_node(), &label);
					if let Some(old) = Names::<T>::get(node) {
						ensure!(Self::status(&old, Self::now()).is_none(), Error::<T>::NameTaken);
						Self::clear_registration(&old);
					}
					let reg = Self::next_reg();
					let account = Self::manager(&owner);
					Names::<T>::insert(
						node,
						NameRecord {
							label: label.clone(),
							parent: kurd_node(),
							parent_reg: 0,
							reg,
							owner: owner.clone(),
							kind: Kind::Granted,
							locked_transfer: true,
							expiry: None,
							account,
							content: None,
							deposit: None,
						},
					);
					Self::deposit_event(Event::Granted { node, parent: kurd_node(), label, owner });
				},
				Some(p) => {
					let parent_rec = Names::<T>::get(p).ok_or(Error::<T>::UnknownName)?;
					ensure!(parent_rec.is_institutional(), Error::<T>::NotInstitutional);
					ensure!(
						Self::effective_status(&parent_rec, Self::now()).is_some(),
						Error::<T>::NotActive
					);
					let label =
						label::validate(raw, label::Use::Sub).map_err(|e| Self::label_error(e))?;
					let node = child_node(&p, &label);
					ensure!(Names::<T>::get(node).is_none(), Error::<T>::NameTaken);
					Self::put_subname(p, &parent_rec, &label[..], owner.clone(), None, None)?;
					Self::deposit_event(Event::Granted { node, parent: p, label, owner });
				},
			}
			Ok(())
		}

		/// Release everything a registration holds except the record itself.
		fn clear_registration(rec: &RecordOf<T>) {
			for (_, (_, payer, held)) in TextRecords::<T>::drain_prefix(rec.reg) {
				let _ = T::Currency::release(
					&HoldReason::Text.into(),
					&payer,
					held,
					Precision::BestEffort,
				);
			}
			TextCount::<T>::remove(rec.reg);
			if let Some((payer, held)) = &rec.deposit {
				let _ = T::Currency::release(
					&HoldReason::Subname.into(),
					payer,
					*held,
					Precision::BestEffort,
				);
			}
		}

		fn delete(node: NameHash, rec: &RecordOf<T>) {
			Self::clear_registration(rec);
			let _ = PendingChanges::<T>::clear_prefix(node, 3, None);
			Names::<T>::remove(node);
		}

		/// Whether `node` currently resolves to `who`.
		pub fn resolves_to(node: NameHash, who: &T::AccountId) -> bool {
			Names::<T>::get(node).is_some_and(|rec| {
				rec.account.as_ref() == Some(who)
					&& Self::effective_status(&rec, Self::now()).is_some()
			})
		}

		/// The node of a dotted name ending in `.kurd`, if every label is valid.
		pub fn node_of(name: &[u8]) -> Option<NameHash> {
			let rest = name.strip_suffix(b".kurd")?;
			let mut node = kurd_node();
			for l in rest.rsplit(|b| *b == b'.') {
				label::validate(l, label::Use::Sub).ok()?;
				node = child_node(&node, l);
			}
			Some(node)
		}

		/// `runtime_api::NavnisanApi::resolve`.
		pub fn api_resolve(
			name: &[u8],
		) -> Option<runtime_api::Resolution<T::AccountId, OfficeOf<T>>> {
			let node = Self::node_of(name)?;
			let rec = Names::<T>::get(node)?;
			let status = Self::effective_status(&rec, Self::now())?;
			Some(runtime_api::Resolution {
				node,
				account: rec.account.clone(),
				content: rec.content.clone(),
				owner: rec.owner.clone(),
				kind: rec.kind,
				status,
				expiry: rec.expiry,
				reg: rec.reg,
				official: rec.is_institutional(),
			})
		}

		pub fn api_text(name: &[u8], key: &[u8]) -> Option<Vec<u8>> {
			let node = Self::node_of(name)?;
			let rec = Names::<T>::get(node)?;
			Self::effective_status(&rec, Self::now())?;
			let key: label::TextKey = key.to_vec().try_into().ok()?;
			TextRecords::<T>::get(rec.reg, key).map(|(v, _, _)| v.into_inner())
		}

		/// The full dotted name of `node`, built from the labels upward.
		pub fn full_name(node: NameHash) -> Option<Vec<u8>> {
			let mut labels = Vec::new();
			let mut cur = node;
			for _ in 0..label::MAX_DEPTH {
				let rec = Names::<T>::get(cur)?;
				labels.push(rec.label.into_inner());
				if rec.parent == kurd_node() {
					labels.push(b"kurd".to_vec());
					return Some(labels.join(&b'.'));
				}
				cur = rec.parent;
			}
			None
		}

		pub fn api_reverse(who: &T::AccountId) -> Option<Vec<u8>> {
			let node = Primary::<T>::get(who)?;
			Self::resolves_to(node, who).then(|| Self::full_name(node)).flatten()
		}

		pub fn api_price(raw: &[u8], years: u32) -> Option<BalanceOf<T>> {
			let l = label::validate(raw, label::Use::Buy).ok()?;
			(years >= 1 && years <= T::MaxYears::get()).then(|| Self::price(l.len(), years))
		}

		pub fn api_availability(raw: &[u8]) -> runtime_api::Availability {
			use runtime_api::Availability as A;
			match label::validate(raw, label::Use::Buy) {
				Err(label::LabelError::TooShortToBuy) => return A::GrantOnly,
				Err(_) => return A::Invalid,
				Ok(_) => {},
			}
			let node = child_node(&kurd_node(), raw);
			if Locked::<T>::contains_key(node) {
				return A::Locked;
			}
			if PendingRevocations::<T>::contains_key(node) {
				return A::Taken;
			}
			match Names::<T>::get(node).and_then(|r| Self::status(&r, Self::now())) {
				Some(Status::Active) => A::Taken,
				Some(Status::Grace) => A::Grace,
				None => A::Free,
			}
		}

		/// Writes the genesis set; also the Zagros seed migration.
		pub fn seed(
			zones: &[Vec<u8>],
			gov_subnames: &[(Vec<u8>, Option<OfficeOf<T>>)],
			locked: &[(Vec<u8>, u8)],
			prices: PriceBands<BalanceOf<T>>,
		) -> DispatchResult {
			ensure!(Self::prices_are_valid(&prices), Error::<T>::BadPrices);
			Prices::<T>::put(prices);
			for z in zones {
				Self::do_grant(None, z, Owner::Governance)?;
			}
			let gov = child_node(&kurd_node(), b"gov");
			if !gov_subnames.is_empty() {
				ensure!(Names::<T>::contains_key(gov), Error::<T>::UnknownName);
			}
			for (l, office) in gov_subnames {
				let owner = match office {
					Some(o) => Owner::Office(*o),
					None => Owner::Governance,
				};
				Self::do_grant(Some(gov), l, owner)?;
			}
			for (l, category) in locked {
				let label =
					label::validate(l, label::Use::Grant).map_err(|e| Self::label_error(e))?;
				Locked::<T>::insert(child_node(&kurd_node(), &label), category);
			}
			Ok(())
		}

		#[cfg(any(feature = "try-runtime", test))]
		pub fn do_try_state() -> Result<(), pezsp_runtime::TryRuntimeError> {
			use pezsp_runtime::TryRuntimeError as E;
			let next = NextRegistration::<T>::get();
			for (node, rec) in Names::<T>::iter() {
				ensure!(
					child_node(&rec.parent, &rec.label) == node,
					E::Other("node is not namehash")
				);
				ensure!(
					label::validate(&rec.label, label::Use::Sub).is_ok(),
					E::Other("bad label")
				);
				ensure!(rec.reg <= next && rec.reg > 0, E::Other("registration out of range"));
				match rec.kind {
					Kind::Leased => ensure!(
						rec.parent == kurd_node()
							&& rec.expiry.is_some()
							&& matches!(rec.owner, Owner::Account(_))
							&& !rec.locked_transfer,
						E::Other("leased record malformed")
					),
					Kind::Granted => ensure!(
						rec.expiry.is_none() && rec.locked_transfer,
						E::Other("granted record malformed")
					),
					Kind::Sub => ensure!(rec.parent != kurd_node(), E::Other("subname at the top")),
				}
				if let Owner::Office(o) = &rec.owner {
					ensure!(T::Offices::is_single_seat(o), E::Other("office not single-seat"));
				}
				let texts = TextRecords::<T>::iter_prefix(rec.reg).count();
				ensure!(texts == TextCount::<T>::get(rec.reg) as usize, E::Other("text count"));
				ensure!(texts as u32 <= label::MAX_TEXTS, E::Other("too many texts"));
			}
			ensure!(
				Self::prices_are_valid(&Prices::<T>::get()) || Names::<T>::iter().next().is_none(),
				E::Other("prices invalid")
			);
			Ok(())
		}
	}
}
