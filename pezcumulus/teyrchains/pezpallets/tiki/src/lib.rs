// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

//! # Tiki (Role) Pezpallet
//!
//! A pezpallet for managing citizenship and role-based NFTs with automated and governance-driven
//! assignment.
//!
//! ## Overview
//!
//! The Tiki pezpallet implements a comprehensive role management system using non-transferable NFTs
//! to represent citizenship status and various roles within the ecosystem. Each role grants
//! specific permissions, rights, and social standing.
//!
//! ## Core Concepts
//!
//! ### Citizenship NFT
//! - Automatically minted upon KYC approval
//! - Represents "Welati" (Citizen) status
//! - Non-transferable and permanent
//! - Required prerequisite for all other roles
//!
//! ### Role Types (Tiki)
//!
//! Roles are assigned through different mechanisms:
//!
//! 1. **Automatic** - System-assigned upon conditions (e.g., Citizenship after KYC)
//! 2. **Appointed** - Admin-assigned governmental positions (e.g., Ministers, Judges)
//! 3. **Elected** - Community-voted positions (e.g., Parliament members)
//! 4. **Earned** - Achievement-based roles (e.g., Educator, Expert)
//!
//! ### Role Categories
//!
//! - **Governance**: Serok (President), SerokWeziran (Prime Minister), Ministers
//! - **Judicial**: Dadger (Judge), Dozger (Prosecutor), Hiquqnas (Lawyer)
//! - **Administrative**: Qeydkar (Registrar), Xezinedar (Treasurer), OperatorêTorê (Network
//!   Operator)
//! - **Educational**: Mamoste (Teacher), Perwerdekar (Educator), Rewsenbîr (Intellectual)
//! - **Economic**: Bazargan (Merchant), Navbeynkar (Mediator)
//! - **Community**: Parlementer (Parliament Member), ModeratorêCivakê (Community Moderator)
//! - **Expert**: Axa (Elder/Expert), Pêseng (Pioneer), Hekem (Wise), Sêwirmend (Counselor)
//!
//! ## NFT Implementation
//!
//! - Built on top of `pezpallet-nfts` for standard NFT functionality
//! - All Tiki NFTs are non-transferable (soulbound)
//! - Transfer attempts are blocked automatically via hooks
//! - Each role is represented by a unique NFT item in the TikiCollectionId
//!
//! ## Role Management
//!
//! ### Granting Roles
//! - Some roles are unique (only one holder at a time)
//! - Users can hold multiple compatible roles
//! - Maximum roles per user is configurable
//!
//! ### Revoking Roles
//! - Admin can revoke appointed roles
//! - Automatic revocation on condition changes
//! - Role history maintained for governance transparency
//!
//! ## Interface
//!
//! ### Extrinsics
//!
//! - `grant_tiki(who, tiki, assignment_type)` - Assign a role to a user (admin)
//! - `revoke_tiki(who, tiki)` - Remove a role from a user (admin)
//! - `grant_honorary_citizenship(who)` - Confer citizenship directly (head of government)
//!
//! ### Storage
//!
//! - `CitizenNft` - Mapping of accounts to their citizenship NFT IDs
//! - `UserTikis` - List of roles held by each user
//! - `TikiHolder` - Reverse mapping for unique roles to their holders
//! - `NextItemId` - Counter for NFT item ID generation
//!
//! ### Soulbound NFTs
//!
//! Every citizen NFT has transfers disabled at mint. There is no hook and no per-block scan:
//! minting is driven by `identity-kyc` through `CitizenNftProvider`.
//!
//! ## Dependencies
//!
//! This pezpallet requires integration with:
//! - `pezpallet-identity-kyc` - KYC status and approval notifications
//! - `pezpallet-nfts` - Underlying NFT infrastructure
//!
//! Note that the dependency runs the other way for trust: this pallet reports a score to
//! `pezpallet-trust` and asks it nothing. No role here is gated on a trust score.
//!
//! ## Runtime Integration Example
//!
//! ```ignore
//! impl pezpallet_tiki::Config for Runtime {
//!     type RuntimeEvent = RuntimeEvent;
//!     type AdminOrigin = EnsureRoot<AccountId>;
//!     // Elected/Earned roles use their own origin, kept distinct from ordinary
//!     // admin appointment so a real election/exam pipeline can gate them later.
//!     type ElectedRoleOrigin = EnsureRoot<AccountId>;
//!     type EarnedRoleOrigin = EnsureRoot<AccountId>;
//!     type WeightInfo = pezpallet_tiki::weights::BizinikiwiWeight<Runtime>;
//!     type TikiCollectionId = ConstU32<1>; // Tiki collection ID
//!     type MaxTikisPerUser = ConstU32<20>; // Max 20 roles per user
//!     type Tiki = pezpallet_tiki::Tiki;
//! }
//! ```

extern crate alloc;

pub use pezpallet::*;

use alloc::{format, vec::Vec};
use pezframe_support::{
	pezpallet_prelude::{MaybeSerializeDeserialize, Parameter},
	traits::Incrementable,
};
use pezsp_runtime::DispatchError;

/// The ceiling a person's combined role bonuses are counted up to.
///
/// The bonuses add up without a natural limit -- somebody could hold a dozen offices and
/// titles at once -- and trust weights this component as a percentage of its maximum. A
/// component with no maximum cannot be a percentage of anything, so one is declared: enough
/// for a full public life several times over (the President's office is 200, the highest
/// earned title 250), and a stated ceiling rather than an accident of who holds what.
pub const MAX_TIKI_SCORE: u32 = 1_000;

/// How a pallet that holds the evidence awards an earned role.
///
/// `Earned` roles -- Axa, Mamoste, Rewsenbîr, Serokê Komelê, Moderatorê Civakê -- are meant to
/// come from having done something: passing courses, bringing citizens in. The pallets that
/// know whether that happened are `perwerde` and `referral`, and neither of them could reach
/// this one: `grant_earned_role` was bound to `EnsureRoot` and had no caller anywhere, so the
/// whole category was granted by nobody, ever.
///
/// Declared here, on the pallet being called, so the caller depends on this pallet and not the
/// reverse -- the same shape `identity-kyc` uses for `OnKycApproved`, and for the same reason.
/// What each threshold is stays with the pallet that measures it.
pub trait EarnedRoleGranter<AccountId, Role> {
	/// Award `role` to `who`. Already holding it is not an error: the caller is reporting that
	/// a threshold was crossed, and a count that crossed it keeps going up.
	fn grant_earned(who: &AccountId, role: Role) -> pezsp_runtime::DispatchResult;
}

/// For runtimes and mocks where nothing awards earned roles.
impl<AccountId, Role> EarnedRoleGranter<AccountId, Role> for () {
	fn grant_earned(_who: &AccountId, _role: Role) -> pezsp_runtime::DispatchResult {
		Ok(())
	}
}

/// Trait for notifying trust score system when tiki score changes.
/// Defined locally to avoid cyclic dependency with pezpallet-trust.
pub trait TrustScoreUpdater<AccountId> {
	fn on_score_component_changed(who: &AccountId);
}

/// Noop implementation for mock environments.
impl<AccountId> TrustScoreUpdater<AccountId> for () {
	fn on_score_component_changed(_who: &AccountId) {}
}
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;
pub use weights::*;
pub mod ensure;
pub mod migrations; // Storage migrations // For origin validation

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_runtime::traits::StaticLookup;

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(migrations::STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config:
		pezframe_system::Config<RuntimeEvent: From<Event<Self>>>
		+ pezpallet_nfts::Config<ItemId = u32>
		+ pezpallet_identity_kyc::Config
	{
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Origin required to grant a role through the election system
		/// (`grant_elected_role`). Deliberately kept distinct from `AdminOrigin`: an
		/// Elected office (Serok, SerokiMeclise, Parlementer, ...) is meant to carry
		/// evidence of a genuine election, not merely ordinary admin/committee say-so.
		/// Runtimes should wire this to whatever origin their election pipeline
		/// escalates to (e.g. `EnsureRoot`), rather than reusing `AdminOrigin`.
		type ElectedRoleOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Origin required to grant a role through the exam/earned system
		/// (`grant_earned_role`). Deliberately kept distinct from `AdminOrigin` for the
		/// same reason as `ElectedRoleOrigin`.
		type EarnedRoleOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Who may grant citizenship directly, without the referral and KYC path.
		///
		/// Honorary citizenship: the state naming someone a citizen because it chooses to.
		/// That is a real power a state has, and it belongs to the head of government rather
		/// than to an administrator -- so the runtime binds this to the `SerokWeziran` tiki,
		/// with root alongside it only for as long as sudo exists.
		///
		/// An honorary citizen counts as a citizen in every way, including towards the
		/// population figures. A state that inflated its own population to reach a milestone
		/// would be lying about itself, and no mechanism here can prevent that better than the
		/// fact that it would be visible on-chain to everyone.
		type HonoraryCitizenshipOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Who may strip an office from someone who was elected to it, or a title someone
		/// earned.
		///
		/// The constitutional court, not the executive and not a committee. An office that
		/// came from a ballot should not be removable by the body it is meant to check --
		/// otherwise a council majority is a coup. What an elected officeholder can be removed
		/// by is a court finding against them, or the next ballot.
		type ImpeachmentOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type WeightInfo: weights::WeightInfo;

		/// Collection ID holding Tiki (Role) NFTs.
		#[pezpallet::constant]
		type TikiCollectionId: Get<Self::CollectionId>;

		/// Technical upper limit for maximum number of Tikis (roles) a user can hold.
		#[pezpallet::constant]
		type MaxTikisPerUser: Get<u32>;

		/// Tiki enum type to be used within the pezpallet.
		type Tiki: Parameter
			+ From<Tiki>
			+ Into<u32>
			+ MaxEncodedLen
			+ TypeInfo
			+ Copy
			+ MaybeSerializeDeserialize
			+ 'static;

		/// Trust score updater - notifies trust pallet when tiki score changes
		type TrustScoreUpdater: TrustScoreUpdater<Self::AccountId>;
	}

	#[derive(
		Serialize,
		Deserialize,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Eq,
		PartialEq,
		Debug,
		TypeInfo,
		MaxEncodedLen,
		Copy,
	)]
	pub enum RoleAssignmentType {
		/// Automatically assigned roles (like Welati after KYC)
		#[codec(index = 0)]
		Automatic,
		/// Admin-assigned roles (like Wezir, Dadger)
		#[codec(index = 1)]
		Appointed,
		/// Community-elected roles (like Parlementer) - assigned by pezpallet-voting
		#[codec(index = 2)]
		Elected,
		/// Earned roles (Axa, roles obtained through exams)
		#[codec(index = 3)]
		Earned,
	}

	#[derive(
		Serialize,
		Deserialize,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Eq,
		PartialEq,
		Debug,
		TypeInfo,
		MaxEncodedLen,
		Copy,
	)]
	#[repr(u32)]
	pub enum Tiki {
		#[codec(index = 0)]
		Welati,
		#[codec(index = 1)]
		Parlementer,
		#[codec(index = 2)]
		SerokiMeclise,
		#[codec(index = 3)]
		Serok,
		#[codec(index = 4)]
		Wezir,
		#[codec(index = 5)]
		EndameDiwane,
		#[codec(index = 6)]
		Dadger,
		#[codec(index = 7)]
		Dozger,
		#[codec(index = 8)]
		Hiquqnas,
		#[codec(index = 9)]
		Noter,
		#[codec(index = 10)]
		Xezinedar,
		#[codec(index = 11)]
		Bacgir,
		#[codec(index = 12)]
		GerinendeyeCavkaniye,
		#[codec(index = 13)]
		OperatorêTorê,
		#[codec(index = 14)]
		PisporêEwlehiyaSîber,
		#[codec(index = 15)]
		GerinendeyeDaneye,
		#[codec(index = 16)]
		Berdevk,
		#[codec(index = 17)]
		Qeydkar,
		#[codec(index = 18)]
		Balyoz,
		#[codec(index = 19)]
		Navbeynkar,
		#[codec(index = 20)]
		ParêzvaneÇandî,
		#[codec(index = 21)]
		Mufetîs,
		#[codec(index = 22)]
		KalîteKontrolker,
		#[codec(index = 23)]
		Mela,
		#[codec(index = 24)]
		Feqî,
		#[codec(index = 25)]
		Perwerdekar,
		#[codec(index = 26)]
		Rewsenbîr,
		#[codec(index = 27)]
		RêveberêProjeyê,
		#[codec(index = 28)]
		SerokêKomele,
		#[codec(index = 29)]
		ModeratorêCivakê,
		#[codec(index = 30)]
		Axa,
		#[codec(index = 31)]
		Pêseng,
		#[codec(index = 32)]
		Sêwirmend,
		#[codec(index = 33)]
		Hekem,
		#[codec(index = 34)]
		Mamoste,
		// Newly added economic roles
		#[codec(index = 35)]
		Bazargan,
		// Government roles
		#[codec(index = 36)]
		SerokWeziran,
		#[codec(index = 37)]
		WezireDarayiye,
		#[codec(index = 38)]
		WezireParez,
		#[codec(index = 39)]
		WezireDad,
		#[codec(index = 40)]
		WezireBelaw,
		#[codec(index = 41)]
		WezireTend,
		#[codec(index = 42)]
		WezireAva,
		#[codec(index = 43)]
		WezireCand,
		// Newly added functional / professional roles. Appended at the end to preserve
		// the SCALE encoding (discriminant order) of existing on-chain values. The trust
		// bonuses assigned below are provisional and should be ratified by governance.
		#[codec(index = 44)]
		Bernamenivîs, // Software developer / engineer (builds the chain itself)
		#[codec(index = 45)]
		Wergêr, // Translator (a six-language nation needs this)
		#[codec(index = 46)]
		Aborînas, // Economist
		#[codec(index = 47)]
		Hesabdar, // Accountant
		#[codec(index = 48)]
		Rojnamevan, // Journalist
		#[codec(index = 49)]
		PisporêBazarkirinê, // Marketing specialist
		#[codec(index = 50)]
		Statîstîknas, // Statistician
		#[codec(index = 51)]
		Piştrastkar, // KYC verifier
		#[codec(index = 52)]
		Hilbijartinkar, // Election officer
		#[codec(index = 53)]
		Îcrakar, // Executor / enforcement officer
		#[codec(index = 54)]
		Karguzar, // Human-resources officer
		#[codec(index = 55)]
		Plansaz, // Budget planner
	}

	impl From<Tiki> for u32 {
		fn from(val: Tiki) -> Self {
			val as u32
		}
	}

	/// Holds citizenship NFT ID for each user
	#[pezpallet::storage]
	#[pezpallet::getter(fn citizen_nft)]
	pub type CitizenNft<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, OptionQuery>;

	/// List of Tikis (roles) owned by each user
	#[pezpallet::storage]
	#[pezpallet::getter(fn user_tikis)]
	pub type UserTikis<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<Tiki, T::MaxTikisPerUser>,
		ValueQuery,
	>;

	/// Shows which user a specific Tiki belongs to (for unique roles)
	#[pezpallet::storage]
	#[pezpallet::getter(fn tiki_holder)]
	pub type TikiHolder<T: Config> =
		StorageMap<_, Blake2_128Concat, Tiki, T::AccountId, OptionQuery>;

	/// Item ID to be used for next NFT
	#[pezpallet::storage]
	#[pezpallet::getter(fn next_item_id)]
	pub type NextItemId<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Records, per (account, tiki), which `RoleAssignmentType` path was used the last
	/// time that role was granted to that account (Automatic/Appointed/Elected/Earned).
	/// This gives an on-chain, queryable audit trail distinguishing a role that came
	/// through `grant_elected_role`/`grant_earned_role` from one granted directly via
	/// `grant_tiki`, even though today all of these entry points share similar origins.
	#[pezpallet::storage]
	#[pezpallet::getter(fn role_assignment_type_of)]
	pub type RoleAssignmentTypeOf<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		Tiki,
		RoleAssignmentType,
		OptionQuery,
	>;

	/// When a role runs out, for the roles that have a term.
	///
	/// The value is written by whoever grants the role, because only they know the term: the
	/// ballot knows how long a parliament sits, the court knows how long a judge serves. What
	/// this pallet contributes is that the term is *enforced* -- an expired role reads as
	/// absent from the moment it expires, whether or not anybody remembered to remove it.
	///
	/// Roles with no entry here have no term. That is the right default: citizenship does not
	/// expire, and neither does a title someone earned.
	#[pezpallet::storage]
	#[pezpallet::getter(fn tiki_expiry)]
	pub type TikiExpiry<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		Tiki,
		BlockNumberFor<T>,
		OptionQuery,
	>;

	#[pezpallet::error]
	pub enum Error<T> {
		/// Role already belongs to someone else
		RoleAlreadyTaken,
		/// Role not assigned
		RoleNotAssigned,
		/// A user has reached maximum role count
		ExceedsMaxRolesPerUser,
		/// KYC not completed
		KycNotCompleted,
		/// Citizenship NFT already exists
		CitizenNftAlreadyExists,
		/// Citizenship NFT not found
		CitizenNftNotFound,
		/// User already has this role
		UserAlreadyHasRole,
		/// This role type cannot be assigned with this method
		InvalidRoleAssignmentMethod,
		/// This role cannot be taken away by anyone.
		RoleNotRevocable,
		/// Another pallet owns the seating of this office; use its path.
		SeatedByGovernance,
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New citizenship NFT minted
		CitizenNftMinted { who: T::AccountId, nft_id: u32 },
		/// The state conferred citizenship on someone directly.
		HonoraryCitizenshipGranted { who: T::AccountId },
		/// New Tiki (role) granted
		TikiGranted { who: T::AccountId, tiki: Tiki },
		/// Tiki (role) revoked
		TikiRevoked { who: T::AccountId, tiki: Tiki },
		/// NFT transfer blocked
		TransferBlocked {
			collection_id: T::CollectionId,
			item_id: u32,
			from: T::AccountId,
			to: T::AccountId,
		},
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		/// The three records of who holds what must agree.
		///
		/// `UserTikis` is the list per person, `TikiHolder` is the reverse index for offices
		/// with one holder, and `RoleAssignmentTypeOf` says how each was granted. They are
		/// written together and can only diverge through a path that updates one and forgets
		/// another -- which is invisible at the call site and shows up later as an office with
		/// two holders, or a holder no lookup can find. Everything downstream reads one of
		/// these: the treasury asks who holds the finance portfolio, the government asks who
		/// is Prime Minister. Whichever one is wrong is the one that decides.
		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use pezframe_support::ensure;

			// Every reverse-index entry points at someone who actually holds the role.
			for (tiki, holder) in TikiHolder::<T>::iter() {
				ensure!(
					Self::is_unique_role(&tiki),
					"TikiHolder has an entry for a role that may have several holders"
				);
				ensure!(
					UserTikis::<T>::get(&holder).contains(&tiki),
					"TikiHolder names someone whose own role list does not have that role"
				);
			}

			for (account, tikis) in UserTikis::<T>::iter() {
				// Holding any role at all means being a citizen.
				ensure!(
					CitizenNft::<T>::get(&account).is_some(),
					"an account holds roles without a citizen NFT"
				);

				let mut seen = alloc::vec::Vec::new();
				for tiki in tikis.iter() {
					ensure!(!seen.contains(tiki), "an account holds the same role twice");
					seen.push(*tiki);

					// A single-holder office must be indexed back to this account, and to
					// nobody else.
					if Self::is_unique_role(tiki) {
						ensure!(
							TikiHolder::<T>::get(tiki).as_ref() == Some(&account),
							"a unique role is held by someone the reverse index does not name"
						);
					}
				}
			}

			// Provenance is recorded for exactly the roles that are held, and says something
			// the role could actually have been granted by.
			for (account, tiki, assignment) in RoleAssignmentTypeOf::<T>::iter() {
				ensure!(
					UserTikis::<T>::get(&account).contains(&tiki),
					"a grant is recorded for a role the account does not hold"
				);
				ensure!(
					Self::can_grant_role_type(&tiki, &assignment),
					"a role records a grant type it cannot be granted by"
				);
			}

			Ok(())
		}
	}
	// Citizenship NFT minting is handled by CitizenNftProvider hooks,
	// no per-block scanning needed.

	// ============= GENESIS CONFIG =============

	/// Genesis configuration for bootstrapping Collection 0 and founding citizen NFT.
	///
	/// When `founding_citizen` is `Some(account)`, genesis will:
	/// 1. Create NFT Collection 0 in pezpallet_nfts (with DepositRequired disabled)
	/// 2. Mint NFT Item #0 for the founding citizen
	/// 3. Populate CitizenNft, NextItemId, and UserTikis storage
	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// Optional founding citizen who receives NFT #0 at genesis.
		/// If None, Collection 0 is NOT created (must be created via sudo later).
		pub founding_citizen: Option<T::AccountId>,

		/// Offices held from the first block.
		///
		/// A state whose offices are all empty on day one cannot do anything, including fill
		/// them: every path that appoints or elects someone is itself gated on an office. The
		/// founding government breaks that circle, and it is deliberately written in the
		/// chain spec rather than granted later by a key -- what genesis says is auditable
		/// before the chain starts, and cannot be quietly changed afterwards.
		///
		/// Each holder is made a citizen first, because no tiki can be granted to someone
		/// without a citizen NFT. Only takes effect when `founding_citizen` is set, since
		/// otherwise Collection 0 does not exist yet.
		pub founding_government: alloc::vec::Vec<(T::AccountId, Tiki)>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			use pezsp_runtime::traits::Zero;

			let collection_id = T::TikiCollectionId::get();

			if let Some(ref founder) = self.founding_citizen {
				// Step 1: Create Collection 0 in pezpallet_nfts
				// Disable DepositRequired so genesis minting doesn't need balance
				let collection_config = pezpallet_nfts::CollectionConfig {
					settings: pezpallet_nfts::CollectionSettings(
						pezpallet_nfts::CollectionSetting::DepositRequired.into(),
					),
					max_supply: None,
					mint_settings: Default::default(),
				};

				pezpallet_nfts::Pezpallet::<T>::do_create_collection(
					collection_id,
					founder.clone(),
					founder.clone(),
					collection_config,
					Zero::zero(),
					pezpallet_nfts::Event::ForceCreated {
						collection: collection_id,
						owner: founder.clone(),
					},
				)
				.expect("Tiki genesis: failed to create Collection 0");

				// `do_create_collection` writes the collection but does not move
				// `NextCollectionId`; every caller inside pallet-nfts advances it separately,
				// and this one has to as well. Without it the chain starts with collection 0
				// occupied and the counter still pointing at 0, so the first attempt to create
				// any other collection fails with `CollectionIdInUse` -- a failure that would
				// surface long after genesis, on whichever call happened to be first.
				//
				// Measured on the live People chain (block 2_132_268): 96 citizen NFTs, tiki's
				// own NextItemId at 96, collection 0 present, and `Nfts::NextCollectionId`
				// never written. Nothing has needed a second collection there yet, which is
				// the only reason it has not been hit.
				pezpallet_nfts::NextCollectionId::<T>::set(collection_id.increment());

				// Step 2: Mint the founder's citizen NFT through the same path every other
				// citizen takes.
				//
				// This used to be a hand-rolled `do_mint` with `ItemSettings::all_enabled()`
				// followed by three storage writes -- which meant NFT #0, the anchor of the
				// whole identity system, was the one citizen NFT that was never locked and so
				// the only one that could be transferred away. Going through
				// `mint_citizen_nft_for_user` mints it, locks it, records it, grants Welati
				// and writes the metadata, exactly as it does for everyone else.
				Pezpallet::<T>::mint_citizen_nft_for_user(founder)
					.expect("Tiki genesis: failed to mint the founder's citizen NFT");

				// Step 3: Seat the founding government.
				for (holder, tiki) in self.founding_government.iter() {
					if CitizenNft::<T>::get(holder).is_none() {
						Pezpallet::<T>::mint_citizen_nft_for_user(holder)
							.expect("Tiki genesis: failed to mint a minister's citizen NFT");
					}
					Pezpallet::<T>::internal_grant_role(holder, *tiki)
						.expect("Tiki genesis: failed to seat the founding government");
					RoleAssignmentTypeOf::<T>::insert(holder, tiki, RoleAssignmentType::Appointed);
				}
			}
		}
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Grant a Tiki (role) to a specific user by an admin
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(<T as crate::pezpallet::Config>::WeightInfo::grant_tiki())]
		pub fn grant_tiki(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			tiki: Tiki,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let dest_account = T::Lookup::lookup(dest)?;

			ensure!(!Self::is_seated_by_governance(&tiki), Error::<T>::SeatedByGovernance);

			// Check if the role can be appointed
			ensure!(
				Self::can_grant_role_type(&tiki, &RoleAssignmentType::Appointed),
				Error::<T>::InvalidRoleAssignmentMethod
			);

			Self::internal_grant_role(&dest_account, tiki)?;
			RoleAssignmentTypeOf::<T>::insert(&dest_account, tiki, RoleAssignmentType::Appointed);
			Ok(())
		}

		/// Remove a Tiki from someone. Who may do it depends on how they got it.
		///
		/// The old version asked `AdminOrigin` for everything, and `AdminOrigin` is Root or
		/// the President or a council majority. So a council majority could strip the tiki
		/// from a President the country had elected. Granting was already bound to the way a
		/// role is obtained; taking it away was not, and an asymmetry there is the whole
		/// difference between a check and a coup.
		///
		/// - `Automatic` (Welati): nobody. Citizenship is removed by `identity-kyc`.
		/// - `Appointed`: the admin origin. Cabinet posts have their own path in `welati`;
		///   this is for the ordinary appointed offices and for correcting mistakes.
		/// - `Elected`: the court. The other way an elected office changes hands is the next
		///   ballot, which does it directly rather than through this call.
		/// - `Earned`: the court. A title someone earned is not an appointment to be
		///   withdrawn; it is taken only by a finding against them.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(<T as crate::pezpallet::Config>::WeightInfo::revoke_tiki())]
		pub fn revoke_tiki(
			origin: OriginFor<T>,
			target: <T::Lookup as StaticLookup>::Source,
			tiki: Tiki,
		) -> DispatchResult {
			let target_account = T::Lookup::lookup(target)?;

			// How it was actually granted, falling back to how it can be granted -- a role
			// seated before provenance was recorded still has a taxonomy.
			let assignment = RoleAssignmentTypeOf::<T>::get(&target_account, tiki)
				.unwrap_or_else(|| Self::get_role_assignment_type(&tiki));

			ensure!(!Self::is_seated_by_governance(&tiki), Error::<T>::SeatedByGovernance);

			match assignment {
				RoleAssignmentType::Automatic => return Err(Error::<T>::RoleNotRevocable.into()),
				RoleAssignmentType::Appointed => {
					T::AdminOrigin::ensure_origin(origin)?;
				},
				RoleAssignmentType::Elected | RoleAssignmentType::Earned => {
					T::ImpeachmentOrigin::ensure_origin(origin)?;
				},
			}

			Self::internal_revoke_role(&target_account, tiki)?;
			Ok(())
		}

		/// Grant honorary citizenship.
		///
		/// The ordinary way in is `identity-kyc`: a citizen vouches for you, you confirm, and
		/// the NFT follows. This is the other way -- the state conferring citizenship on
		/// someone it wants to honour, with no referrer and no application.
		///
		/// It used to be `AdminOrigin`, described as "for testing/emergency", which meant a
		/// council majority could hand out citizenship. It is now the head of government's,
		/// which is where the power to name honorary citizens actually sits.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(<T as crate::pezpallet::Config>::WeightInfo::grant_honorary_citizenship())]
		pub fn grant_honorary_citizenship(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			T::HonoraryCitizenshipOrigin::ensure_origin(origin)?;
			let dest_account = T::Lookup::lookup(dest)?;

			Self::mint_citizen_nft_for_user(&dest_account)?;
			// And tell the citizen register, which is what the treasury counts and every
			// election reads. Minting the NFT alone would make them a citizen here and a
			// stranger there.
			pezpallet_identity_kyc::Pezpallet::<T>::register_honorary_citizen(&dest_account)?;
			Self::deposit_event(Event::HonoraryCitizenshipGranted { who: dest_account });
			Ok(())
		}

		/// Grant role through election system (called from pezpallet-voting)
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(<T as crate::pezpallet::Config>::WeightInfo::grant_elected_role())]
		pub fn grant_elected_role(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			tiki: Tiki,
		) -> DispatchResult {
			// Distinct from AdminOrigin: an Elected role must come through the
			// election-specific origin (see `ElectedRoleOrigin` doc comment).
			T::ElectedRoleOrigin::ensure_origin(origin)?;
			let dest_account = T::Lookup::lookup(dest)?;

			// Check if the role can be granted through election
			ensure!(
				Self::can_grant_role_type(&tiki, &RoleAssignmentType::Elected),
				Error::<T>::InvalidRoleAssignmentMethod
			);

			Self::internal_grant_role(&dest_account, tiki)?;
			RoleAssignmentTypeOf::<T>::insert(&dest_account, tiki, RoleAssignmentType::Elected);
			Ok(())
		}

		/// Grant role through exam/test system
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(<T as crate::pezpallet::Config>::WeightInfo::grant_earned_role())]
		pub fn grant_earned_role(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			tiki: Tiki,
		) -> DispatchResult {
			// Distinct from AdminOrigin: an Earned role must come through the
			// exam-specific origin (see `EarnedRoleOrigin` doc comment).
			T::EarnedRoleOrigin::ensure_origin(origin)?;
			let dest_account = T::Lookup::lookup(dest)?;

			// Check if the role can be earned
			ensure!(
				Self::can_grant_role_type(&tiki, &RoleAssignmentType::Earned),
				Error::<T>::InvalidRoleAssignmentMethod
			);

			Self::internal_grant_role(&dest_account, tiki)?;
			RoleAssignmentTypeOf::<T>::insert(&dest_account, tiki, RoleAssignmentType::Earned);
			Ok(())
		}

		/// Apply for citizenship after KYC completion
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(<T as crate::pezpallet::Config>::WeightInfo::apply_for_citizenship())]
		pub fn apply_for_citizenship(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Check if user's KYC is approved
			let kyc_status = pezpallet_identity_kyc::Pezpallet::<T>::kyc_status_of(&who);
			ensure!(
				kyc_status == pezpallet_identity_kyc::types::KycLevel::Approved,
				Error::<T>::KycNotCompleted
			);

			// Mint citizenship NFT
			Self::mint_citizen_nft_for_user(&who)?;

			Ok(())
		}

		// `check_transfer_permission` used to live here. It was an extrinsic that ignored its
		// own origin, was called from nowhere, and returned an error saying citizen NFTs are
		// non-transferable -- which read as the mechanism that made them so. It was not. The
		// guard is `disable_transfer`, applied to every citizen NFT the moment it is minted;
		// `transfers_are_refused` in the tests is what holds it.
	}

	// Pezpallet's helper functions
	impl<T: Config> Pezpallet<T> {
		/// Mints citizenship NFT for specific user
		pub fn mint_citizen_nft_for_user(user: &T::AccountId) -> DispatchResult {
			// Check if NFT already exists
			ensure!(Self::citizen_nft(user).is_none(), Error::<T>::CitizenNftAlreadyExists);

			let collection_id = T::TikiCollectionId::get();
			let next_id_u32 = Self::next_item_id();

			// Mint the NFT via force_mint (root origin, no deposit needed)
			pezpallet_nfts::Pezpallet::<T>::force_mint(
				T::RuntimeOrigin::from(pezframe_system::RawOrigin::Root),
				collection_id,
				next_id_u32,
				T::Lookup::unlookup(user.clone()),
				Default::default(),
			)?;

			// Make NFT non-transferable
			Self::lock_nft_transfer(&collection_id, &next_id_u32)?;

			// Update storage
			CitizenNft::<T>::insert(user, next_id_u32);
			NextItemId::<T>::put(next_id_u32.saturating_add(1));

			// Automatically add Welati role.
			//
			// This used to discard the result. A full role list would then mint the NFT, write
			// `CitizenNft`, and leave the person a citizen with no Welati tiki -- which every
			// other pallet reads as "not a citizen" while this one says they are.
			UserTikis::<T>::try_mutate(user, |tikis| {
				tikis.try_push(Tiki::Welati).map_err(|_| Error::<T>::ExceedsMaxRolesPerUser)
			})?;
			RoleAssignmentTypeOf::<T>::insert(user, Tiki::Welati, RoleAssignmentType::Automatic);

			// Set NFT metadata
			Self::update_nft_metadata(user)?;

			Self::deposit_event(Event::CitizenNftMinted { who: user.clone(), nft_id: next_id_u32 });
			Ok(())
		}

		/// Grant a role that runs out at `ends_at`.
		///
		/// Called by whoever knows the term. Everything else is the ordinary grant.
		pub fn internal_grant_role_until(
			dest_account: &T::AccountId,
			tiki: Tiki,
			ends_at: BlockNumberFor<T>,
		) -> DispatchResult {
			Self::internal_grant_role(dest_account, tiki)?;
			TikiExpiry::<T>::insert(dest_account, tiki, ends_at);
			Ok(())
		}

		/// Internal role granting function (to avoid code duplication)
		pub fn internal_grant_role(dest_account: &T::AccountId, tiki: Tiki) -> DispatchResult {
			// Check if citizenship NFT exists
			ensure!(Self::citizen_nft(dest_account).is_some(), Error::<T>::CitizenNftNotFound);

			// A single-holder office can only be granted if it is free. An office whose term
			// has run out counts as free: otherwise the one thing a term is supposed to make
			// possible -- replacing someone whose time is up -- would be the one thing it
			// blocks.
			if Self::is_unique_role(&tiki) {
				if let Some(holder) = TikiHolder::<T>::get(tiki) {
					ensure!(Self::has_expired(&holder, &tiki), Error::<T>::RoleAlreadyTaken);
					Self::internal_revoke_role(&holder, tiki)?;
				}
			}

			// Check if user already has this role
			let user_tikis = Self::user_tikis(dest_account);
			ensure!(!user_tikis.contains(&tiki), Error::<T>::UserAlreadyHasRole);

			// Add to user's Tiki list
			UserTikis::<T>::try_mutate(dest_account, |tikis| {
				tikis.try_push(tiki).map_err(|_| Error::<T>::ExceedsMaxRolesPerUser)
			})?;

			// If unique role, also add to TikiHolder
			if Self::is_unique_role(&tiki) {
				TikiHolder::<T>::insert(tiki, dest_account);
			}

			// Update NFT metadata
			Self::update_nft_metadata(dest_account)?;

			Self::deposit_event(Event::TikiGranted { who: dest_account.clone(), tiki });

			// Notify trust pallet that user's tiki score component changed
			T::TrustScoreUpdater::on_score_component_changed(dest_account);

			Ok(())
		}

		/// Internal role revocation function
		pub fn internal_revoke_role(target_account: &T::AccountId, tiki: Tiki) -> DispatchResult {
			// Check if user has this role
			let user_tikis = Self::user_tikis(target_account);
			let _position =
				user_tikis.iter().position(|&r| r == tiki).ok_or(Error::<T>::RoleNotAssigned)?;

			// Welati role cannot be removed
			ensure!(tiki != Tiki::Welati, Error::<T>::RoleNotAssigned);

			// Remove from user's Tiki list
			UserTikis::<T>::mutate(target_account, |tikis| {
				if let Some(pos) = tikis.iter().position(|&r| r == tiki) {
					tikis.swap_remove(pos);
				}
			});

			// If unique role, also remove from TikiHolder
			if Self::is_unique_role(&tiki) {
				TikiHolder::<T>::remove(tiki);
			}

			// Clear the recorded assignment-type provenance and the term for this pair.
			RoleAssignmentTypeOf::<T>::remove(target_account, tiki);
			TikiExpiry::<T>::remove(target_account, tiki);

			// Update NFT metadata
			Self::update_nft_metadata(target_account)?;

			Self::deposit_event(Event::TikiRevoked { who: target_account.clone(), tiki });

			// Notify trust pallet that user's tiki score component changed
			T::TrustScoreUpdater::on_score_component_changed(target_account);

			Ok(())
		}

		/// Makes NFT non-transferable using the system-level TransferDisabled attribute.
		/// This sets PalletAttributes::TransferDisabled which is checked by pezpallet_nfts
		/// during transfer operations, providing a proper soulbound guarantee.
		pub(crate) fn lock_nft_transfer(
			collection_id: &T::CollectionId,
			item_id: &u32,
		) -> DispatchResult {
			use pezframe_support::traits::tokens::nonfungibles_v2::Transfer;
			pezpallet_nfts::Pezpallet::<T>::disable_transfer(collection_id, item_id)
		}

		/// Lift the soulbound lock. Only ever used immediately before burning.
		pub(crate) fn unlock_nft_transfer(
			collection_id: &T::CollectionId,
			item_id: &u32,
		) -> DispatchResult {
			use pezframe_support::traits::tokens::nonfungibles_v2::Transfer;
			pezpallet_nfts::Pezpallet::<T>::enable_transfer(collection_id, item_id)
		}

		/// Updates NFT metadata based on user's roles
		fn update_nft_metadata(user: &T::AccountId) -> DispatchResult {
			let nft_id_u32 = Self::citizen_nft(user).ok_or(Error::<T>::CitizenNftNotFound)?;
			let collection_id = T::TikiCollectionId::get();
			let user_tikis = Self::user_tikis(user);

			let total_score = Self::get_tiki_score(user);

			// Short metadata - only basic information
			let metadata = format!(
				r#"{{"citizen":true,"roles":{},"score":{}}}"#,
				user_tikis.len(),
				total_score
			);

			// Set metadata - log error but don't crash
			if pezpallet_nfts::Pezpallet::<T>::set_metadata(
				T::RuntimeOrigin::from(pezframe_system::RawOrigin::Root),
				collection_id,
				nft_id_u32,
				metadata
					.as_bytes()
					.to_vec()
					.try_into()
					.map_err(|_| DispatchError::Other("Metadata too long"))?,
			)
			.is_err()
			{
				log::warn!("Failed to set metadata for NFT: {nft_id_u32:?}");
			}

			Ok(())
		}

		/// Whether an office may have only one holder at a time.
		///
		/// The line is the office, not the seniority. `Xezinedar` -- the central bank
		/// governor -- is one post and one person; `Dadger` is a judge, and a state has as
		/// many judges as it needs. The same reading explains the rest: the cabinet posts are
		/// single portfolios, while `Wezir`, `Parlementer` and the professional titles are
		/// descriptions of what someone does, which any number of people can do.
		///
		/// It matters beyond bookkeeping. A unique role blocks a second grant, so listing
		/// something here that should not be caps the state at one of them; leaving out
		/// something that should be here lets an office quietly acquire a second holder, with
		/// `TikiHolder` naming only one of them and every lookup answering differently
		/// depending on which record it reads.
		pub fn is_unique_role(tiki: &Tiki) -> bool {
			matches!(
				tiki,
				Tiki::Serok
					| Tiki::SerokiMeclise
					| Tiki::Xezinedar
					| Tiki::Balyoz
					// Government roles: single-portfolio cabinet offices, each of which
					// must have at most one holder at a time (see get_bonus_for_tiki,
					// which grants each of these a fixed per-office trust bonus).
					| Tiki::SerokWeziran
					| Tiki::WezireDarayiye
					| Tiki::WezireParez
					| Tiki::WezireDad
					| Tiki::WezireBelaw
					| Tiki::WezireTend
					| Tiki::WezireAva
					| Tiki::WezireCand
			)
		}

		/// Whether `tiki` is a cabinet portfolio.
		///
		/// The list lives here because this pallet is the register these offices are written
		/// into. A second copy in the pallet that seats them would be free to drift from the
		/// one the guard below reads, and the guard would then protect a different set than
		/// the appointer is allowed to fill.
		pub fn is_cabinet_tiki(tiki: &Tiki) -> bool {
			matches!(
				tiki,
				Tiki::WezireDarayiye
					| Tiki::WezireParez
					| Tiki::WezireDad
					| Tiki::WezireBelaw
					| Tiki::WezireTend
					| Tiki::WezireAva
					| Tiki::WezireCand
					| Tiki::Wezir
			)
		}

		/// Whether a constitutional process, rather than this pallet's admin call, seats `tiki`.
		///
		/// The cabinet is the Prime Minister's to fill, the Prime Minister is the President's,
		/// and the bench is filled by the house and the President under the court's own rules.
		/// Each of those paths writes into this register through the internal functions.
		///
		/// `grant_tiki` and `revoke_tiki` are the ordinary door, and they answer to a wider
		/// origin than any of those paths do. Without this guard the register could assert an
		/// office no constitutional process decided: a council majority could seat a Prime
		/// Minister the President never appointed, or mint a judge who never sat -- and the
		/// forged seat carries the trust score of a real one.
		///
		/// Offices nobody else seats -- Xezinedar, Balyoz -- are deliberately absent: for them
		/// this pallet's admin call is the only door there is, and closing it would leave the
		/// office unfillable.
		pub fn is_seated_by_governance(tiki: &Tiki) -> bool {
			Self::is_cabinet_tiki(tiki) || matches!(tiki, Tiki::SerokWeziran | Tiki::EndameDiwane)
		}

		/// Returns the assignment type of a specific role
		pub fn get_role_assignment_type(tiki: &Tiki) -> RoleAssignmentType {
			match tiki {
				// Automatic roles
				Tiki::Welati => RoleAssignmentType::Automatic,

				// Elected roles
				Tiki::Parlementer | Tiki::SerokiMeclise | Tiki::Serok => {
					RoleAssignmentType::Elected
				},

				// Earned roles (automatically given by pezpallet-referral)
				Tiki::Axa
				| Tiki::Mamoste
				| Tiki::Rewsenbîr
				| Tiki::SerokêKomele
				| Tiki::ModeratorêCivakê => RoleAssignmentType::Earned,

				// Appointed roles (default)
				_ => RoleAssignmentType::Appointed,
			}
		}

		/// Checks the granting method of a specific role
		pub fn can_grant_role_type(tiki: &Tiki, assignment_type: &RoleAssignmentType) -> bool {
			let required_type = Self::get_role_assignment_type(tiki);
			match (&required_type, assignment_type) {
				// Automatic roles can only be given by the system
				(RoleAssignmentType::Automatic, RoleAssignmentType::Automatic) => true,
				// Appointed roles can be given by admin
				(RoleAssignmentType::Appointed, RoleAssignmentType::Appointed) => true,
				// Elected roles can be given by election system
				(RoleAssignmentType::Elected, RoleAssignmentType::Elected) => true,
				// Earned roles can be given by exam/test system
				(RoleAssignmentType::Earned, RoleAssignmentType::Earned) => true,
				_ => false,
			}
		}

		/// Automatically grant the Welati role after KYC
		pub fn auto_grant_citizenship(account: &T::AccountId) -> DispatchResult {
			// KYC check
			let kyc_status = pezpallet_identity_kyc::Pezpallet::<T>::kyc_status_of(account);
			if kyc_status == pezpallet_identity_kyc::types::KycLevel::Approved {
				// Mint the citizenship NFT if it does not exist
				if Self::citizen_nft(account).is_none() {
					Self::mint_citizen_nft_for_user(account)?;
				}
			}
			Ok(())
		}

		/// Checks whether the user holds a specific Tiki
		pub fn has_tiki(who: &T::AccountId, tiki: &Tiki) -> bool {
			Self::user_tikis(who).contains(tiki) && !Self::has_expired(who, tiki)
		}

		/// Whether a role's term has run out.
		///
		/// Roles with no term never expire, which is why the default is `false`.
		pub fn has_expired(who: &T::AccountId, tiki: &Tiki) -> bool {
			match TikiExpiry::<T>::get(who, tiki) {
				Some(ends_at) => pezframe_system::Pezpallet::<T>::block_number() > ends_at,
				None => false,
			}
		}

		/// Who holds a single-holder office right now, or nobody if the term has run out.
		///
		/// This is what other pallets should ask, not the raw `TikiHolder` map. The difference
		/// is exactly the case the term exists for: an officeholder whose term ended and whom
		/// nobody got around to removing. Reading the map directly would hand them the office
		/// indefinitely -- the failure a term is meant to prevent.
		pub fn current_holder(tiki: &Tiki) -> Option<T::AccountId> {
			TikiHolder::<T>::get(tiki).filter(|holder| !Self::has_expired(holder, tiki))
		}

		/// Checks whether the user is a citizen
		pub fn is_citizen(who: &T::AccountId) -> bool {
			Self::citizen_nft(who).is_some()
		}
	}
}

/// Trait used by other pallets to query Tiki scores from this pallet
pub trait TikiScoreProvider<AccountId> {
	fn get_tiki_score(who: &AccountId) -> u32;

	/// The ceiling the score is counted up to. See `MAX_TIKI_SCORE`.
	fn max_score() -> u32 {
		MAX_TIKI_SCORE
	}
}

/// Trait used by other pallets to query Tiki ownership
pub trait TikiProvider<AccountId> {
	fn has_tiki(who: &AccountId, tiki: &Tiki) -> bool;
	fn get_user_tikis(who: &AccountId) -> Vec<Tiki>;
	fn is_citizen(who: &AccountId) -> bool;
}

/// Trait implementations
impl<T: Config> TikiScoreProvider<T::AccountId> for Pezpallet<T> {
	fn get_tiki_score(who: &T::AccountId) -> u32 {
		Self::user_tikis(who)
			.iter()
			// An office whose term has ended stops counting towards standing the moment it
			// ends, not whenever someone gets around to removing it.
			.filter(|tiki| !Self::has_expired(who, tiki))
			.map(Self::get_bonus_for_tiki)
			.fold(0u32, |acc, x| acc.saturating_add(x))
			.min(MAX_TIKI_SCORE)
	}

	fn max_score() -> u32 {
		MAX_TIKI_SCORE
	}
}

impl<T: Config> EarnedRoleGranter<T::AccountId, Tiki> for Pezpallet<T> {
	fn grant_earned(who: &T::AccountId, tiki: Tiki) -> pezsp_runtime::DispatchResult {
		use pezframe_support::ensure;

		// Only for roles the taxonomy says are earned. A pallet that counts referrals has no
		// business handing out a judgeship, and this is the line that says so.
		ensure!(
			Self::can_grant_role_type(&tiki, &RoleAssignmentType::Earned),
			Error::<T>::InvalidRoleAssignmentMethod
		);

		// Crossing a threshold again is not a failure; it is the ordinary case, since the
		// count that crossed it keeps going up.
		if Self::has_tiki(who, &tiki) {
			return Ok(());
		}

		Self::internal_grant_role(who, tiki)?;
		RoleAssignmentTypeOf::<T>::insert(who, tiki, RoleAssignmentType::Earned);
		Ok(())
	}
}

impl<T: Config> TikiProvider<T::AccountId> for Pezpallet<T> {
	fn has_tiki(who: &T::AccountId, tiki: &Tiki) -> bool {
		Self::has_tiki(who, tiki)
	}

	fn get_user_tikis(who: &T::AccountId) -> Vec<Tiki> {
		Self::user_tikis(who).into_inner()
	}

	fn is_citizen(who: &T::AccountId) -> bool {
		Self::is_citizen(who)
	}
}

// Keeping the scoring logic in a separate impl block to keep the code more organized.
impl<T: Config> Pezpallet<T> {
	/// Returns the contribution of a specific Tiki to the Trust Score.
	pub fn get_bonus_for_tiki(tiki: &Tiki) -> u32 {
		match tiki {
			// Special scores defined in Anayasa v5.0
			Tiki::Axa => 250,
			Tiki::RêveberêProjeyê => 250,
			Tiki::ModeratorêCivakê => 200,
			Tiki::SerokêKomele => 100,
			Tiki::Mela => 50,
			Tiki::Feqî => 50,

			// Hierarchical State Scores
			// Judiciary
			Tiki::EndameDiwane => 175,
			Tiki::Dadger => 150,
			Tiki::Dozger => 120,
			Tiki::Hiquqnas => 75,
			// Executive
			Tiki::Serok => 200,
			Tiki::Wezir => 100,
			Tiki::SerokWeziran => 125,
			Tiki::WezireDarayiye => 100,
			Tiki::WezireParez => 100,
			Tiki::WezireDad => 100,
			Tiki::WezireBelaw => 100,
			Tiki::WezireTend => 100,
			Tiki::WezireAva => 100,
			Tiki::WezireCand => 100,

			// Legislature
			Tiki::SerokiMeclise => 150,
			Tiki::Parlementer => 100,

			// Appointed Senior Officials
			Tiki::Xezinedar => 100,
			Tiki::PisporêEwlehiyaSîber => 100,
			Tiki::Mufetîs => 90,
			Tiki::Balyoz => 80,
			Tiki::Berdevk => 70,

			// Other Officials and Experts
			Tiki::Mamoste => 70,
			Tiki::OperatorêTorê => 60,
			Tiki::Noter => 50,
			Tiki::Bacgir => 50,
			Tiki::Perwerdekar => 40,
			Tiki::Rewsenbîr => 40,
			Tiki::GerinendeyeCavkaniye => 40,
			Tiki::GerinendeyeDaneye => 40,
			Tiki::KalîteKontrolker => 30,
			Tiki::Navbeynkar => 30,
			Tiki::Hekem => 30,
			Tiki::Qeydkar => 25,
			Tiki::ParêzvaneÇandî => 25,
			Tiki::Sêwirmend => 20,
			Tiki::Bazargan => 60,
			Tiki::Pêseng => 80,

			// Newly added functional / professional roles (provisional bonuses — ratify via governance)
			Tiki::Bernamenivîs => 80,
			Tiki::Aborînas => 75,
			Tiki::Plansaz => 70,
			Tiki::Piştrastkar => 60,
			Tiki::Hilbijartinkar => 60,
			Tiki::Îcrakar => 60,
			Tiki::Wergêr => 50,
			Tiki::Hesabdar => 50,
			Tiki::Rojnamevan => 50,
			Tiki::Statîstîknas => 50,
			Tiki::PisporêBazarkirinê => 40,
			Tiki::Karguzar => 40,

			// Basic Citizenship
			Tiki::Welati => 10,
		}
	}
}
// CitizenNftProvider trait implementation for pezpallet-identity-kyc integration
impl<T: Config> pezpallet_identity_kyc::types::CitizenNftProvider<T::AccountId> for Pezpallet<T> {
	fn mint_citizen_nft(who: &T::AccountId) -> pezsp_runtime::DispatchResult {
		Self::mint_citizen_nft_for_user(who)
	}

	fn mint_citizen_nft_confirmed(who: &T::AccountId) -> pezsp_runtime::DispatchResult {
		// For self-confirmation, we use the same mint function with force_mint
		Self::mint_citizen_nft_for_user(who)
	}

	/// Strip someone of citizenship: their offices first, then the NFT.
	///
	/// The order is the whole point. This used to burn the NFT and clear the roles afterwards,
	/// and `identity-kyc::revoke_citizenship` only logs a failure here rather than reverting.
	/// So a burn that failed for any reason -- and it can, since it dispatches into
	/// `pezpallet-nfts` as the holder -- left someone recorded as `Revoked` who was still the
	/// finance minister. Clearing the offices first means a failure leaves an orphaned NFT
	/// instead of an orphaned authority, which is the direction a failure should go.
	fn burn_citizen_nft(who: &T::AccountId) -> pezsp_runtime::DispatchResult {
		use pezframe_support::traits::Get;

		let item_id = Self::citizen_nft(who).ok_or(Error::<T>::CitizenNftNotFound)?;
		let collection_id = T::TikiCollectionId::get();

		let user_tikis = UserTikis::<T>::get(who);
		for tiki in user_tikis.iter() {
			if Self::is_unique_role(tiki) {
				TikiHolder::<T>::remove(tiki);
			}
			RoleAssignmentTypeOf::<T>::remove(who, tiki);
			TikiExpiry::<T>::remove(who, tiki);
		}
		UserTikis::<T>::remove(who);
		CitizenNft::<T>::remove(who);

		// The NFT is soulbound, and `pezpallet-nfts` refuses to burn an item whose transfers
		// are disabled -- `ItemLocked`, from the same attribute that makes it soulbound. So
		// every burn attempted here has always failed. `renounce_citizenship` propagates that
		// with `?`, which means no citizen has ever been able to leave; `revoke_citizenship`
		// swallowed it, which means a revoked citizen kept the NFT.
		//
		// The lock has to come off for the burn and go back on if the burn does not happen.
		// The window is inside one call and the account already holds no offices by this
		// point, so a transferable NFT here would be a worthless one.
		Self::unlock_nft_transfer(&collection_id, &item_id)?;
		if let Err(e) = pezpallet_nfts::Pezpallet::<T>::burn(
			T::RuntimeOrigin::from(pezframe_system::RawOrigin::Signed(who.clone())),
			collection_id,
			item_id,
		) {
			let _ = Self::lock_nft_transfer(&collection_id, &item_id);
			return Err(e);
		}

		Ok(())
	}
}

// ===== STORED ENUM ENCODING =====
//
// SCALE encodes a fieldless enum by the variant's position, and three of these are storage
// keys. Insert a variant in the middle -- grouping by ministry, or alphabetising, is the most
// natural thing anyone would do -- and every key already written decodes as a different
// value. It does not break; it quietly means something else. A judge becomes a treasurer.
//
// The explicit indices pin the number to the variant rather than to its position, and this
// holds those numbers to what they were when the chain started. A variant may be added at the
// end with the next free number; nothing here may be renumbered, and a number left behind by
// a removed variant is not reusable.
//
// Generating those indices is itself the hazard this guards against: the first attempt lost
// nineteen variants whose names carry Kurdish letters and silently shifted everything after
// them. Two of the shifts collided and the codec derive refused to compile; the rest would
// have gone through.

#[cfg(test)]
mod stored_enum_encoding {
	use super::*;
	use codec::Encode;

	#[test]
	fn tiki_indices_are_pinned() {
		let pinned: &[(&str, u8, &dyn Fn() -> Vec<u8>)] = &[
			("Welati", 0u8, &|| Tiki::Welati.encode()),
			("Parlementer", 1u8, &|| Tiki::Parlementer.encode()),
			("SerokiMeclise", 2u8, &|| Tiki::SerokiMeclise.encode()),
			("Serok", 3u8, &|| Tiki::Serok.encode()),
			("Wezir", 4u8, &|| Tiki::Wezir.encode()),
			("EndameDiwane", 5u8, &|| Tiki::EndameDiwane.encode()),
			("Dadger", 6u8, &|| Tiki::Dadger.encode()),
			("Dozger", 7u8, &|| Tiki::Dozger.encode()),
			("Hiquqnas", 8u8, &|| Tiki::Hiquqnas.encode()),
			("Noter", 9u8, &|| Tiki::Noter.encode()),
			("Xezinedar", 10u8, &|| Tiki::Xezinedar.encode()),
			("Bacgir", 11u8, &|| Tiki::Bacgir.encode()),
			("GerinendeyeCavkaniye", 12u8, &|| Tiki::GerinendeyeCavkaniye.encode()),
			("OperatorêTorê", 13u8, &|| Tiki::OperatorêTorê.encode()),
			("PisporêEwlehiyaSîber", 14u8, &|| Tiki::PisporêEwlehiyaSîber.encode()),
			("GerinendeyeDaneye", 15u8, &|| Tiki::GerinendeyeDaneye.encode()),
			("Berdevk", 16u8, &|| Tiki::Berdevk.encode()),
			("Qeydkar", 17u8, &|| Tiki::Qeydkar.encode()),
			("Balyoz", 18u8, &|| Tiki::Balyoz.encode()),
			("Navbeynkar", 19u8, &|| Tiki::Navbeynkar.encode()),
			("ParêzvaneÇandî", 20u8, &|| Tiki::ParêzvaneÇandî.encode()),
			("Mufetîs", 21u8, &|| Tiki::Mufetîs.encode()),
			("KalîteKontrolker", 22u8, &|| Tiki::KalîteKontrolker.encode()),
			("Mela", 23u8, &|| Tiki::Mela.encode()),
			("Feqî", 24u8, &|| Tiki::Feqî.encode()),
			("Perwerdekar", 25u8, &|| Tiki::Perwerdekar.encode()),
			("Rewsenbîr", 26u8, &|| Tiki::Rewsenbîr.encode()),
			("RêveberêProjeyê", 27u8, &|| Tiki::RêveberêProjeyê.encode()),
			("SerokêKomele", 28u8, &|| Tiki::SerokêKomele.encode()),
			("ModeratorêCivakê", 29u8, &|| Tiki::ModeratorêCivakê.encode()),
			("Axa", 30u8, &|| Tiki::Axa.encode()),
			("Pêseng", 31u8, &|| Tiki::Pêseng.encode()),
			("Sêwirmend", 32u8, &|| Tiki::Sêwirmend.encode()),
			("Hekem", 33u8, &|| Tiki::Hekem.encode()),
			("Mamoste", 34u8, &|| Tiki::Mamoste.encode()),
			("Bazargan", 35u8, &|| Tiki::Bazargan.encode()),
			("SerokWeziran", 36u8, &|| Tiki::SerokWeziran.encode()),
			("WezireDarayiye", 37u8, &|| Tiki::WezireDarayiye.encode()),
			("WezireParez", 38u8, &|| Tiki::WezireParez.encode()),
			("WezireDad", 39u8, &|| Tiki::WezireDad.encode()),
			("WezireBelaw", 40u8, &|| Tiki::WezireBelaw.encode()),
			("WezireTend", 41u8, &|| Tiki::WezireTend.encode()),
			("WezireAva", 42u8, &|| Tiki::WezireAva.encode()),
			("WezireCand", 43u8, &|| Tiki::WezireCand.encode()),
			("Bernamenivîs", 44u8, &|| Tiki::Bernamenivîs.encode()),
			("Wergêr", 45u8, &|| Tiki::Wergêr.encode()),
			("Aborînas", 46u8, &|| Tiki::Aborînas.encode()),
			("Hesabdar", 47u8, &|| Tiki::Hesabdar.encode()),
			("Rojnamevan", 48u8, &|| Tiki::Rojnamevan.encode()),
			("PisporêBazarkirinê", 49u8, &|| Tiki::PisporêBazarkirinê.encode()),
			("Statîstîknas", 50u8, &|| Tiki::Statîstîknas.encode()),
			("Piştrastkar", 51u8, &|| Tiki::Piştrastkar.encode()),
			("Hilbijartinkar", 52u8, &|| Tiki::Hilbijartinkar.encode()),
			("Îcrakar", 53u8, &|| Tiki::Îcrakar.encode()),
			("Karguzar", 54u8, &|| Tiki::Karguzar.encode()),
			("Plansaz", 55u8, &|| Tiki::Plansaz.encode()),
		];
		let moved: Vec<&str> = pinned
			.iter()
			.filter(|(_, want, enc)| enc() != vec![*want])
			.map(|(name, _, _)| *name)
			.collect();
		assert!(moved.is_empty(), "`Tiki` indices moved: {moved:?}");
		assert_eq!(pinned.len(), 56, "a variant was added or removed");
	}

	#[test]
	fn roleassignmenttype_indices_are_pinned() {
		let pinned: &[(&str, u8, &dyn Fn() -> Vec<u8>)] = &[
			("Automatic", 0u8, &|| RoleAssignmentType::Automatic.encode()),
			("Appointed", 1u8, &|| RoleAssignmentType::Appointed.encode()),
			("Elected", 2u8, &|| RoleAssignmentType::Elected.encode()),
			("Earned", 3u8, &|| RoleAssignmentType::Earned.encode()),
		];
		let moved: Vec<&str> = pinned
			.iter()
			.filter(|(_, want, enc)| enc() != vec![*want])
			.map(|(name, _, _)| *name)
			.collect();
		assert!(moved.is_empty(), "`RoleAssignmentType` indices moved: {moved:?}");
		assert_eq!(pinned.len(), 4, "a variant was added or removed");
	}
}
