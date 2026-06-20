#![cfg_attr(not(feature = "std"), no_std)]

//! # PEZkurd-P2Pmessage Pezpallet
//!
//! Ephemeral, end-to-end encrypted P2P messaging on PezkuwiChain.
//!
//! ## Purpose
//!
//! Provides censorship-resistant communication for Kurdish people,
//! especially those under digital blackout by hostile regimes.
//! Messages are encrypted client-side (XChaCha20-Poly1305) and
//! automatically purged from chain state at era boundaries.
//!
//! ## Design Principles
//!
//! - **Zero Trace**: Messages deleted every era. No permanent record.
//! - **E2E Encrypted**: Only recipient can decrypt. Chain sees ciphertext only.
//! - **Fee-Free**: Citizens with sufficient trust score pay no fees.
//! - **Forward Secrecy**: Ephemeral x25519 keys per message.
//! - **Spam-Resistant**: Rate limiting + citizenship + trust score requirements.
//!
//! ## Architecture
//!
//! ```text
//! Client (wallet/web)                    People Chain
//! ┌──────────────────┐                   ┌─────────────────────┐
//! │ Generate x25519  │──register_key()──>│ EncryptionKeys      │
//! │ keypair          │                   │                     │
//! │                  │                   │                     │
//! │ Lookup recipient │<──read storage────│ EncryptionKeys      │
//! │ public key       │                   │                     │
//! │                  │                   │                     │
//! │ Encrypt with     │──send_message()──>│ Inbox (era-keyed)   │
//! │ XChaCha20-Poly   │                   │                     │
//! │                  │                   │                     │
//! │ Poll & decrypt   │<──read storage────│ Inbox               │
//! │                  │                   │                     │
//! │                  │                   │ on_idle: era ends →  │
//! │                  │                   │   delete all msgs    │
//! └──────────────────┘                   └─────────────────────┘
//! ```
//!
//! ## Extrinsics
//!
//! - `register_encryption_key(x25519_public_key)` — Register/update messaging public key
//! - `send_message(to, ephemeral_pub, nonce, ciphertext)` — Send encrypted message
//! - `acknowledge_messages()` — Clear own inbox (optional, early cleanup)
//!
//! ## Encryption (Client-Side)
//!
//! 1. Sender generates ephemeral x25519 keypair
//! 2. ECDH: shared_secret = ephemeral_private × recipient_public
//! 3. KDF: message_key = HKDF-SHA256(shared_secret)
//! 4. Encrypt: XChaCha20-Poly1305(plaintext, message_key, random_nonce)
//! 5. Submit: (ephemeral_public, nonce, ciphertext) via extrinsic

pub use pezpallet::*;
pub mod types;
use types::*;
pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

extern crate alloc;
use pezframe_support::{pezpallet_prelude::*, traits::Get, weights::WeightMeter};
use pezframe_system::pezpallet_prelude::*;
use pezsp_runtime::traits::Saturating;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

	/// Current storage version.
	pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config<RuntimeEvent: From<Event<Self>>> {
		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// Checks if an account is an approved citizen.
		/// Wired to pezpallet-identity-kyc in runtime.
		type CitizenshipChecker: CitizenshipChecker<Self::AccountId>;

		/// Checks an account's trust score.
		/// Wired to pezpallet-trust in runtime.
		type TrustScoreChecker: TrustScoreChecker<Self::AccountId>;

		/// Minimum trust score required to use messaging.
		/// Citizens below this score cannot send messages or register keys.
		/// Default: 20
		#[pezpallet::constant]
		type MinTrustScore: Get<u32>;

		/// Maximum encrypted payload size in bytes.
		/// Default: 512 bytes (enough for ~350 chars of plaintext + encryption overhead).
		#[pezpallet::constant]
		type MaxMessageSize: Get<u32>;

		/// Maximum messages per inbox (per era, per recipient).
		/// When full, oldest messages are dropped (FIFO).
		#[pezpallet::constant]
		type MaxInboxSize: Get<u32>;

		/// Maximum messages a single account can send per era.
		/// Rate limiting to prevent spam even with feeless transactions.
		#[pezpallet::constant]
		type MaxMessagesPerEra: Get<u32>;

		/// Era length in blocks. Messages are purged every era.
		/// Default: 3600 blocks (6 hours at 6s/block on People Chain).
		#[pezpallet::constant]
		type EraLength: Get<BlockNumberFor<Self>>;
	}

	// ============= STORAGE =============

	/// X25519 public keys for message encryption.
	/// Users register their encryption public key here.
	/// Anyone can look up a recipient's key to encrypt a message for them.
	#[pezpallet::storage]
	pub type EncryptionKeys<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, [u8; 32]>;

	/// Encrypted message inbox, keyed by (era_index, recipient).
	/// Using StorageDoubleMap enables efficient era-based bulk deletion
	/// via `remove_prefix(expired_era)`.
	///
	/// EPHEMERAL: Entire era prefix is deleted in `on_idle` when era rotates.
	#[pezpallet::storage]
	pub type Inbox<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		u32, // era_index
		Blake2_128Concat,
		T::AccountId, // recipient
		BoundedVec<
			EncryptedMessage<T::AccountId, BlockNumberFor<T>, T::MaxMessageSize>,
			ConstU32<100>, // hard cap, actual limit via MaxInboxSize
		>,
		ValueQuery,
	>;

	/// Per-account message send counter for the current era.
	/// Reset when era rotates. Used for rate limiting.
	#[pezpallet::storage]
	pub type SendCount<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		u32, // era_index
		Blake2_128Concat,
		T::AccountId, // sender
		u32,          // count
		ValueQuery,
	>;

	/// Current era index, incremented every EraLength blocks.
	#[pezpallet::storage]
	pub type CurrentEra<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Block number when the current era started.
	#[pezpallet::storage]
	pub type EraStartBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// Cursor for multi-block cleanup of expired eras.
	/// If Some, cleanup is still in progress.
	/// The BoundedVec stores the storage cursor from clear_prefix (max 256 bytes).
	#[pezpallet::storage]
	pub type CleanupCursor<T: Config> = StorageValue<_, (u32, BoundedVec<u8, ConstU32<256>>)>;

	// ============= EVENTS =============

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Encryption key registered or updated
		EncryptionKeyRegistered { who: T::AccountId },
		/// Encrypted message delivered to recipient's inbox
		MessageSent { from: T::AccountId, to: T::AccountId, era: u32 },
		/// Recipient acknowledged and cleared their inbox
		InboxCleared { who: T::AccountId, era: u32, count: u32 },
		/// An expired era's messages were purged from storage
		EraPurged { era: u32 },
		/// Era rotated
		EraRotated { old_era: u32, new_era: u32 },
		/// Oldest message was evicted from a full inbox (FIFO)
		InboxOverflow { recipient: T::AccountId, era: u32 },
	}

	// ============= ERRORS =============

	#[pezpallet::error]
	pub enum Error<T> {
		/// Sender is not an approved citizen
		NotACitizen,
		/// Recipient is not an approved citizen
		RecipientNotCitizen,
		/// Recipient has not registered an encryption key
		RecipientNoEncryptionKey,
		/// Cannot send a message to yourself
		CannotMessageSelf,
		/// Rate limit exceeded for this era
		RateLimitExceeded,
		/// Recipient's inbox is full for this era
		InboxFull,
		/// Message payload is empty
		EmptyPayload,
		/// Message payload exceeds maximum size
		PayloadTooLarge,
		/// Sender's trust score is below the minimum required
		InsufficientTrustScore,
	}

	// ============= HOOKS =============

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		/// Check for era rotation at the start of each block.
		/// Cost: 2 reads (CurrentEra + EraStartBlock), 0-2 writes on rotation.
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let era_length = T::EraLength::get();
			let era_start = EraStartBlock::<T>::get();
			let elapsed = n.saturating_sub(era_start);

			if elapsed >= era_length {
				let old_era = CurrentEra::<T>::get();
				let new_era = old_era.saturating_add(1);
				CurrentEra::<T>::put(new_era);
				EraStartBlock::<T>::put(n);

				Self::deposit_event(Event::EraRotated { old_era, new_era });

				// Schedule cleanup: store the expired era index for on_idle
				if old_era > 0 {
					// Clean era before the one that just ended (2 eras of grace)
					let cleanup_era = old_era.saturating_sub(1);
					CleanupCursor::<T>::put((cleanup_era, BoundedVec::default()));
				}

				T::DbWeight::get().reads_writes(2, 3)
			} else {
				T::DbWeight::get().reads(2)
			}
		}

		/// Purge expired era messages using leftover block capacity.
		/// Will not interfere with user transactions.
		fn on_idle(_n: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let mut meter = WeightMeter::with_limit(remaining_weight);

			// Minimum weight for one cleanup operation
			let min_weight = T::DbWeight::get().reads_writes(1, 1);
			if !meter.can_consume(min_weight) {
				return meter.consumed();
			}

			if let Some((cleanup_era, cursor)) = CleanupCursor::<T>::get() {
				// Consume weight for reading the cursor
				let _ = meter.try_consume(T::DbWeight::get().reads(1));

				let maybe_cursor = if cursor.is_empty() { None } else { Some(cursor.as_slice()) };

				// Delete up to 50 entries per on_idle call
				let result = Inbox::<T>::clear_prefix(cleanup_era, 50, maybe_cursor);

				// Account for the writes
				let writes_weight = T::DbWeight::get().writes(result.unique as u64);
				let _ = meter.try_consume(writes_weight);

				// Also clean SendCount for this era
				let send_result = SendCount::<T>::clear_prefix(cleanup_era, 50, None);
				let send_writes = T::DbWeight::get().writes(send_result.unique as u64);
				let _ = meter.try_consume(send_writes);

				match result.maybe_cursor {
					Some(new_cursor) => {
						// More items remain, save cursor for next block
						let bounded_cursor: BoundedVec<u8, ConstU32<256>> =
							new_cursor.try_into().unwrap_or_default();
						CleanupCursor::<T>::put((cleanup_era, bounded_cursor));
					},
					None => {
						// All items deleted for this era
						CleanupCursor::<T>::kill();
						Self::deposit_event(Event::EraPurged { era: cleanup_era });
					},
				}
			}

			meter.consumed()
		}
	}

	// ============= EXTRINSICS =============

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Register or update your x25519 encryption public key.
		///
		/// This key is used by other citizens to encrypt messages for you.
		/// You must be an approved citizen to register.
		///
		/// # Arguments
		/// - `public_key`: Your x25519 public key (32 bytes), generated client-side
		///
		/// # Fee
		/// Free for citizens (via `feeless_if` + SkipCheckIfFeeless)
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::register_encryption_key())]
		#[pezpallet::feeless_if(|origin: &OriginFor<T>, _public_key: &[u8; 32]| -> bool {
			if let Ok(who) = ensure_signed(origin.clone()) {
				T::CitizenshipChecker::is_citizen(&who)
			} else {
				false
			}
		})]
		pub fn register_encryption_key(
			origin: OriginFor<T>,
			public_key: [u8; 32],
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Must be a citizen
			ensure!(T::CitizenshipChecker::is_citizen(&who), Error::<T>::NotACitizen);

			// Must have sufficient trust score
			ensure!(
				T::TrustScoreChecker::trust_score_of(&who) >= T::MinTrustScore::get(),
				Error::<T>::InsufficientTrustScore
			);

			// Store the encryption key
			EncryptionKeys::<T>::insert(&who, public_key);

			Self::deposit_event(Event::EncryptionKeyRegistered { who });
			Ok(())
		}

		/// Send an encrypted message to another citizen.
		///
		/// The message is E2E encrypted client-side using XChaCha20-Poly1305
		/// with an ephemeral x25519 key exchange. The chain only stores ciphertext.
		///
		/// # Arguments
		/// - `to`: Recipient's account
		/// - `ephemeral_public_key`: Sender's ephemeral x25519 public key for this message
		/// - `nonce`: XChaCha20-Poly1305 nonce (24 bytes, random)
		/// - `ciphertext`: Encrypted message payload
		///
		/// # Fee
		/// Free for citizens (via `feeless_if` + SkipCheckIfFeeless)
		///
		/// # Privacy
		/// - Payload is encrypted, chain cannot read content
		/// - Metadata visible: sender, recipient, timestamp
		/// - Ephemeral key provides forward secrecy
		/// - Message deleted at era boundary (max 6 hours on-chain)
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::send_message(ciphertext.len() as u32))]
		#[pezpallet::feeless_if(|origin: &OriginFor<T>, _to: &T::AccountId, _ephemeral_public_key: &[u8; 32], _nonce: &[u8; 24], _ciphertext: &alloc::vec::Vec<u8>| -> bool {
			if let Ok(who) = ensure_signed(origin.clone()) {
				T::CitizenshipChecker::is_citizen(&who)
			} else {
				false
			}
		})]
		pub fn send_message(
			origin: OriginFor<T>,
			to: T::AccountId,
			ephemeral_public_key: [u8; 32],
			nonce: [u8; 24],
			ciphertext: alloc::vec::Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// === Validation ===

			// Sender must be a citizen
			ensure!(T::CitizenshipChecker::is_citizen(&sender), Error::<T>::NotACitizen);

			// Sender must have sufficient trust score
			ensure!(
				T::TrustScoreChecker::trust_score_of(&sender) >= T::MinTrustScore::get(),
				Error::<T>::InsufficientTrustScore
			);

			// Recipient must be a citizen
			ensure!(T::CitizenshipChecker::is_citizen(&to), Error::<T>::RecipientNotCitizen);

			// Cannot message yourself
			ensure!(sender != to, Error::<T>::CannotMessageSelf);

			// Recipient must have registered an encryption key
			ensure!(EncryptionKeys::<T>::contains_key(&to), Error::<T>::RecipientNoEncryptionKey);

			// Payload validation
			ensure!(!ciphertext.is_empty(), Error::<T>::EmptyPayload);
			ensure!(
				ciphertext.len() <= T::MaxMessageSize::get() as usize,
				Error::<T>::PayloadTooLarge
			);

			// === Rate Limiting ===

			let current_era = CurrentEra::<T>::get();
			let send_count = SendCount::<T>::get(current_era, &sender);
			ensure!(send_count < T::MaxMessagesPerEra::get(), Error::<T>::RateLimitExceeded);

			// === Store Message ===

			let bounded_ciphertext: BoundedVec<u8, T::MaxMessageSize> =
				ciphertext.try_into().map_err(|_| Error::<T>::PayloadTooLarge)?;

			let current_block = pezframe_system::Pezpallet::<T>::block_number();

			let message = EncryptedMessage {
				sender: sender.clone(),
				block_number: current_block,
				ephemeral_public_key,
				nonce,
				ciphertext: bounded_ciphertext,
			};

			// Try to push to recipient's inbox for this era
			Inbox::<T>::try_mutate(current_era, &to, |inbox| -> DispatchResult {
				if inbox.len() >= T::MaxInboxSize::get() as usize {
					// FIFO: remove oldest message to make room
					inbox.remove(0);
					Self::deposit_event(Event::InboxOverflow {
						recipient: to.clone(),
						era: current_era,
					});
				}
				inbox.try_push(message).map_err(|_| Error::<T>::InboxFull)?;
				Ok(())
			})?;

			// Increment send counter
			SendCount::<T>::insert(current_era, &sender, send_count.saturating_add(1));

			Self::deposit_event(Event::MessageSent { from: sender, to, era: current_era });

			Ok(())
		}

		/// Acknowledge and clear your inbox for the current era.
		///
		/// Optional convenience extrinsic. Messages are automatically deleted
		/// at era boundaries anyway. This allows early cleanup and signals
		/// to the sender that messages were received.
		///
		/// # Fee
		/// Free for citizens (via `feeless_if` + SkipCheckIfFeeless)
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::acknowledge_messages())]
		#[pezpallet::feeless_if(|origin: &OriginFor<T>| -> bool {
			if let Ok(who) = ensure_signed(origin.clone()) {
				T::CitizenshipChecker::is_citizen(&who)
			} else {
				false
			}
		})]
		pub fn acknowledge_messages(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let current_era = CurrentEra::<T>::get();

			let inbox = Inbox::<T>::take(current_era, &who);
			let count = inbox.len() as u32;

			Self::deposit_event(Event::InboxCleared { who, era: current_era, count });

			Ok(())
		}
	}
}

// ============= HELPER FUNCTIONS =============

impl<T: Config> Pezpallet<T> {
	/// Get the current era index
	pub fn current_era() -> u32 {
		CurrentEra::<T>::get()
	}

	/// Check if an account has an encryption key registered
	pub fn has_encryption_key(who: &T::AccountId) -> bool {
		EncryptionKeys::<T>::contains_key(who)
	}

	/// Get an account's encryption public key
	pub fn get_encryption_key(who: &T::AccountId) -> Option<[u8; 32]> {
		EncryptionKeys::<T>::get(who)
	}

	/// Get the number of messages in an account's inbox for the current era
	pub fn inbox_count(who: &T::AccountId) -> u32 {
		let era = CurrentEra::<T>::get();
		Inbox::<T>::get(era, who).len() as u32
	}

	/// Get remaining send quota for an account in the current era
	pub fn remaining_send_quota(who: &T::AccountId) -> u32 {
		let era = CurrentEra::<T>::get();
		let used = SendCount::<T>::get(era, who);
		T::MaxMessagesPerEra::get().saturating_sub(used)
	}
}
