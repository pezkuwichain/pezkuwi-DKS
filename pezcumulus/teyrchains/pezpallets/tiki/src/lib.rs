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
//! - Trust score requirements for certain roles
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
//! - `force_mint_citizen_nft(who)` - Manually mint citizenship NFT (admin)
//!
//! ### Storage
//!
//! - `CitizenNft` - Mapping of accounts to their citizenship NFT IDs
//! - `UserTikis` - List of roles held by each user
//! - `TikiHolder` - Reverse mapping for unique roles to their holders
//! - `NextItemId` - Counter for NFT item ID generation
//!
//! ### Hooks
//!
//! - `on_initialize` - Automatic citizenship NFT minting for newly approved KYC users
//! - NFT transfer blocking for all Tiki NFTs
//!
//! ## Dependencies
//!
//! This pezpallet requires integration with:
//! - `pezpallet-identity-kyc` - KYC status and approval notifications
//! - `pezpallet-nfts` - Underlying NFT infrastructure
//! - `pezpallet-trust` - Trust score verification for role eligibility
//!
//! ## Runtime Integration Example
//!
//! ```ignore
//! impl pezpallet_tiki::Config for Runtime {
//!     type RuntimeEvent = RuntimeEvent;
//!     type AdminOrigin = EnsureRoot<AccountId>;
//!     type WeightInfo = pezpallet_tiki::weights::BizinikiwiWeight<Runtime>;
//!     type TikiCollectionId = ConstU32<1>; // Tiki collection ID
//!     type MaxTikisPerUser = ConstU32<20>; // Max 20 roles per user
//!     type Tiki = pezpallet_tiki::Tiki;
//! }
//! ```

extern crate alloc;

pub use pezpallet::*;

use alloc::{format, vec::Vec};
use pezframe_support::pezpallet_prelude::{MaybeSerializeDeserialize, Parameter, RuntimeDebug};
use pezsp_runtime::DispatchError;

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
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
		Copy,
	)]
	pub enum RoleAssignmentType {
		/// Automatically assigned roles (like Welati after KYC)
		Automatic,
		/// Admin-assigned roles (like Wezir, Dadger)
		Appointed,
		/// Community-elected roles (like Parlementer) - assigned by pezpallet-voting
		Elected,
		/// Earned roles (Axa, roles obtained through exams)
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
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
		Copy,
	)]
	#[repr(u32)]
	pub enum Tiki {
		Welati,
		Parlementer,
		SerokiMeclise,
		Serok,
		Wezir,
		EndameDiwane,
		Dadger,
		Dozger,
		Hiquqnas,
		Noter,
		Xezinedar,
		Bacgir,
		GerinendeyeCavkaniye,
		OperatorêTorê,
		PisporêEwlehiyaSîber,
		GerinendeyeDaneye,
		Berdevk,
		Qeydkar,
		Balyoz,
		Navbeynkar,
		ParêzvaneÇandî,
		Mufetîs,
		KalîteKontrolker,
		Mela,
		Feqî,
		Perwerdekar,
		Rewsenbîr,
		RêveberêProjeyê,
		SerokêKomele,
		ModeratorêCivakê,
		Axa,
		Pêseng,
		Sêwirmend,
		Hekem,
		Mamoste,
		// Newly added economic roles
		Bazargan,
		// Government roles
		SerokWeziran,
		WezireDarayiye,
		WezireParez,
		WezireDad,
		WezireBelaw,
		WezireTend,
		WezireAva,
		WezireCand,
		// Newly added functional / professional roles. Appended at the end to preserve
		// the SCALE encoding (discriminant order) of existing on-chain values. The trust
		// bonuses assigned below are provisional and should be ratified by governance.
		Bernamenivîs,       // Software developer / engineer (builds the chain itself)
		Wergêr,             // Translator (a six-language nation needs this)
		Aborînas,           // Economist
		Hesabdar,           // Accountant
		Rojnamevan,         // Journalist
		PisporêBazarkirinê, // Marketing specialist
		Statîstîknas,       // Statistician
		Piştrastkar,        // KYC verifier
		Hilbijartinkar,     // Election officer
		Îcrakar,            // Executor / enforcement officer
		Karguzar,           // Human-resources officer
		Plansaz,            // Budget planner
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

	#[pezpallet::error]
	pub enum Error<T> {
		/// Role already belongs to someone else
		RoleAlreadyTaken,
		/// Specified person is not the holder of this role
		NotTheHolder,
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
		/// Insufficient Trust Score
		InsufficientTrustScore,
		/// This role type cannot be assigned with this method
		InvalidRoleAssignmentMethod,
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New citizenship NFT minted
		CitizenNftMinted { who: T::AccountId, nft_id: u32 },
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
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {}
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

				// Step 2: Mint NFT #0 for the founding citizen
				let item_config = pezpallet_nfts::ItemConfig {
					settings: pezpallet_nfts::ItemSettings::all_enabled(),
				};

				pezpallet_nfts::Pezpallet::<T>::do_mint(
					collection_id,
					0u32,
					None,
					founder.clone(),
					item_config,
					|_, _| Ok(()),
				)
				.expect("Tiki genesis: failed to mint NFT #0");

				// Step 3: Update Tiki storage
				CitizenNft::<T>::insert(founder, 0u32);
				NextItemId::<T>::put(1u32);
				UserTikis::<T>::mutate(founder, |tikis| {
					let _ = tikis.try_push(Tiki::Welati);
				});
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

			// Check if the role can be appointed
			ensure!(
				Self::can_grant_role_type(&tiki, &RoleAssignmentType::Appointed),
				Error::<T>::InvalidRoleAssignmentMethod
			);

			Self::internal_grant_role(&dest_account, tiki)?;
			Ok(())
		}

		/// Remove a Tiki (role) from a specific user by an admin
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(<T as crate::pezpallet::Config>::WeightInfo::revoke_tiki())]
		pub fn revoke_tiki(
			origin: OriginFor<T>,
			target: <T::Lookup as StaticLookup>::Source,
			tiki: Tiki,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let target_account = T::Lookup::lookup(target)?;

			Self::internal_revoke_role(&target_account, tiki)?;
			Ok(())
		}

		/// Manually mint citizenship NFT (for testing/emergency)
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(<T as crate::pezpallet::Config>::WeightInfo::force_mint_citizen_nft())]
		pub fn force_mint_citizen_nft(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let dest_account = T::Lookup::lookup(dest)?;

			Self::mint_citizen_nft_for_user(&dest_account)?;
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
			T::AdminOrigin::ensure_origin(origin)?; // pezpallet-voting will call with Root origin
			let dest_account = T::Lookup::lookup(dest)?;

			// Check if the role can be granted through election
			ensure!(
				Self::can_grant_role_type(&tiki, &RoleAssignmentType::Elected),
				Error::<T>::InvalidRoleAssignmentMethod
			);

			Self::internal_grant_role(&dest_account, tiki)?;
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
			T::AdminOrigin::ensure_origin(origin)?; // For now admin, later exam pezpallet
			let dest_account = T::Lookup::lookup(dest)?;

			// Check if the role can be earned
			ensure!(
				Self::can_grant_role_type(&tiki, &RoleAssignmentType::Earned),
				Error::<T>::InvalidRoleAssignmentMethod
			);

			Self::internal_grant_role(&dest_account, tiki)?;
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

		/// Check NFT transfer for transfer blocking system
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(<T as crate::pezpallet::Config>::WeightInfo::check_transfer_permission())]
		pub fn check_transfer_permission(
			_origin: OriginFor<T>,
			collection_id: T::CollectionId,
			item_id: u32,
			from: T::AccountId,
			to: T::AccountId,
		) -> DispatchResult {
			// If it is the Tiki NFT collection, do not allow the transfer
			if collection_id == T::TikiCollectionId::get() {
				Self::deposit_event(Event::TransferBlocked { collection_id, item_id, from, to });
				return Err(DispatchError::Other("Citizen NFTs are non-transferable"));
			}
			Ok(())
		}
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

			// Automatically add Welati role
			UserTikis::<T>::mutate(user, |tikis| {
				let _ = tikis.try_push(Tiki::Welati);
			});

			// Set NFT metadata
			Self::update_nft_metadata(user)?;

			Self::deposit_event(Event::CitizenNftMinted { who: user.clone(), nft_id: next_id_u32 });
			Ok(())
		}

		/// Internal role granting function (to avoid code duplication)
		pub fn internal_grant_role(dest_account: &T::AccountId, tiki: Tiki) -> DispatchResult {
			// Check if citizenship NFT exists
			ensure!(Self::citizen_nft(dest_account).is_some(), Error::<T>::CitizenNftNotFound);

			// If this role is unique (can belong to only one person), check
			if Self::is_unique_role(&tiki) {
				ensure!(Self::tiki_holder(tiki).is_none(), Error::<T>::RoleAlreadyTaken);
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
		fn lock_nft_transfer(collection_id: &T::CollectionId, item_id: &u32) -> DispatchResult {
			use pezframe_support::traits::tokens::nonfungibles_v2::Transfer;
			pezpallet_nfts::Pezpallet::<T>::disable_transfer(collection_id, item_id)
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

		/// Checks if a specific role is unique (can belong to only one person)
		pub fn is_unique_role(tiki: &Tiki) -> bool {
			matches!(tiki, Tiki::Serok | Tiki::SerokiMeclise | Tiki::Xezinedar | Tiki::Balyoz)
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
			Self::user_tikis(who).contains(tiki)
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
		let tikis = Self::user_tikis(who);
		tikis
			.iter()
			.map(Self::get_bonus_for_tiki)
			.fold(0u32, |acc, x| acc.saturating_add(x))
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

	fn burn_citizen_nft(who: &T::AccountId) -> pezsp_runtime::DispatchResult {
		use pezframe_support::traits::Get;
		// Get the citizen NFT item ID
		let item_id = Self::citizen_nft(who).ok_or(Error::<T>::CitizenNftNotFound)?;
		let collection_id = T::TikiCollectionId::get();

		// Burn the NFT using pezpallet_nfts burn function
		pezpallet_nfts::Pezpallet::<T>::burn(
			T::RuntimeOrigin::from(pezframe_system::RawOrigin::Signed(who.clone())),
			collection_id,
			item_id,
		)?;

		// Clear unique role mappings before removing roles
		let user_tikis = UserTikis::<T>::get(who);
		for tiki in user_tikis.iter() {
			if Self::is_unique_role(tiki) {
				TikiHolder::<T>::remove(tiki);
			}
		}

		// Remove all roles and citizen NFT mapping
		UserTikis::<T>::remove(who);
		CitizenNft::<T>::remove(who);

		Ok(())
	}
}
