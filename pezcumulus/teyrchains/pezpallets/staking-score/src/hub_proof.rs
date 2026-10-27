// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Reading a stake out of the Asset Hub's own state, through proofs anchored in the relay.
//!
//! The relay keeps every teyrchain's latest head, and a head carries that chain's state root.
//! So from a relay storage root this chain trusts -- its own relay parent -- one proof reaches
//! the Asset Hub's head, and a second reaches the hub's staking records under that head's root.
//! Nobody is believed: a proof either hashes to the root or it does not. Whoever submits it
//! can only choose which true thing to show.
//!
//! What is read is decoded with the few leading fields it needs and no more. The layouts are
//! the hub's, so they are pinned by tests against proofs the live chains produced.

use codec::{Decode, Encode};
use pezframe_support::{Blake2_128Concat, PalletId, StorageHasher, Twox128, Twox64Concat};
use pezsp_core::H256;
use pezsp_runtime::traits::BlakeTwo256;
use pezsp_state_machine::{Backend, TrieBackendBuilder};
use pezsp_std::vec::Vec;
use pezsp_trie::StorageProof;

/// Why a proof was turned away.
#[derive(Debug, PartialEq, Eq)]
pub enum ProofError {
	/// The nodes do not reach the root, or do not cover a key that was asked for.
	Incomplete,
	/// A value was there and did not decode as the hub's type.
	Undecodable,
	/// The relay records no head for the hub.
	NoHubHead,
}

/// What the hub says `who` has at stake.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct HubStake {
	/// Bonded directly, plus the share of a nomination pool's bond that is theirs.
	pub staked: u128,
	pub nominations: u32,
	pub unlocking_chunks: u32,
}

fn map_key<H: StorageHasher<Output = Vec<u8>>>(pallet: &[u8], item: &[u8], key: &[u8]) -> Vec<u8> {
	let mut k = Twox128::hash(pallet).to_vec();
	k.extend_from_slice(&Twox128::hash(item));
	k.extend_from_slice(&H::hash(key));
	k
}

struct Reader {
	backend: pezsp_state_machine::TrieBackend<pezsp_trie::MemoryDB<BlakeTwo256>, BlakeTwo256>,
}

impl Reader {
	fn new(root: H256, nodes: &[Vec<u8>]) -> Self {
		let db = StorageProof::new(nodes.iter().cloned()).into_memory_db::<BlakeTwo256>();
		Self { backend: TrieBackendBuilder::new(db, root).build() }
	}

	/// The value at `key`, `None` if the proof shows there is none. A key the proof does not
	/// cover is an error, never an absence: an absence is a claim too, and it has to be proven.
	fn read<V: Decode>(&self, key: &[u8]) -> Result<Option<V>, ProofError> {
		match self.backend.storage(key).map_err(|_| ProofError::Incomplete)? {
			None => Ok(None),
			Some(raw) => V::decode(&mut &raw[..]).map(Some).map_err(|_| ProofError::Undecodable),
		}
	}
}

/// The Asset Hub's state root, as the relay recorded its head under `relay_root`.
pub fn hub_state_root(
	relay_root: H256,
	relay_proof: &[Vec<u8>],
	hub_para_id: u32,
) -> Result<H256, ProofError> {
	let key = map_key::<Twox64Concat>(b"Paras", b"Heads", &hub_para_id.encode());
	// `HeadData` is a byte vector holding the encoded header.
	let head: Vec<u8> =
		Reader::new(relay_root, relay_proof).read(&key)?.ok_or(ProofError::NoHubHead)?;
	let header = pezsp_runtime::generic::Header::<u32, BlakeTwo256>::decode(&mut &head[..])
		.map_err(|_| ProofError::Undecodable)?;
	Ok(header.state_root)
}

/// `pezpallet_staking_async::StakingLedger`, its leading fields.
#[derive(Decode)]
struct Ledger {
	stash: [u8; 32],
	#[codec(compact)]
	_total: u128,
	#[codec(compact)]
	active: u128,
	unlocking: Vec<UnlockChunk>,
}

#[derive(Decode)]
struct UnlockChunk {
	#[codec(compact)]
	_value: u128,
	#[codec(compact)]
	_era: u32,
}

/// `pezpallet_staking_async::Nominations`, its leading field.
#[derive(Decode)]
struct Nominations {
	targets: Vec<[u8; 32]>,
}

/// `pezpallet_nomination_pools::PoolMember`, its leading fields.
#[derive(Decode)]
struct PoolMember {
	pool_id: u32,
	points: u128,
}

/// `pezpallet_nomination_pools::BondedPoolInner` up to `points`. The commission comes first and
/// has to be read through to reach them.
#[derive(Decode)]
struct BondedPoolInner {
	_commission: Commission,
	_member_counter: u32,
	points: u128,
}

#[derive(Decode)]
struct Commission {
	_current: Option<(u32, [u8; 32])>,
	_max: Option<u32>,
	_change_rate: Option<(u32, u32)>,
	_throttle_from: Option<u32>,
	_claim_permission: Option<ClaimPermission>,
}

#[derive(Decode)]
#[allow(dead_code)]
enum ClaimPermission {
	_Permissionless,
	_Account([u8; 32]),
}

/// The account a pool's bond sits in, derived as the hub's pools pallet derives it.
pub fn pool_bonded_account(pools: PalletId, pool_id: u32) -> [u8; 32] {
	let mut account = [0u8; 32];
	account[..4].copy_from_slice(b"modl");
	account[4..12].copy_from_slice(&pools.0);
	// `(AccountType::Bonded, pool_id)`: the variant's index, then the id.
	account[12] = 0;
	account[13..17].copy_from_slice(&pool_id.to_le_bytes());
	account
}

/// The stake bonded under `stash` directly: its ledger's active balance and unlocking chunks,
/// zero if it has none or the ledger names another stash.
fn direct(reader: &Reader, stash: &[u8; 32]) -> Result<(u128, u32), ProofError> {
	let controller: Option<[u8; 32]> =
		reader.read(&map_key::<Twox64Concat>(b"Staking", b"Bonded", stash))?;
	let Some(controller) = controller else { return Ok((0, 0)) };
	let ledger: Option<Ledger> =
		reader.read(&map_key::<Blake2_128Concat>(b"Staking", b"Ledger", &controller))?;
	Ok(match ledger {
		Some(l) if &l.stash == stash => (l.active, l.unlocking.len() as u32),
		_ => (0, 0),
	})
}

/// What `who` has at stake on the hub, read from `hub_proof` under `hub_root`.
pub fn hub_stake(
	hub_root: H256,
	hub_proof: &[Vec<u8>],
	who: &[u8; 32],
	pools: PalletId,
) -> Result<HubStake, ProofError> {
	let reader = Reader::new(hub_root, hub_proof);

	let (direct_active, unlocking_chunks) = direct(&reader, who)?;
	let nominations = reader
		.read::<Nominations>(&map_key::<Twox64Concat>(b"Staking", b"Nominators", who))?
		.map_or(0, |n| n.targets.len() as u32);

	// A pool member's share: their points over the pool's, of the pool's bond.
	let member: Option<PoolMember> =
		reader.read(&map_key::<Twox64Concat>(b"NominationPools", b"PoolMembers", who))?;
	let pooled = match member {
		None => 0,
		Some(m) => {
			let pool: Option<BondedPoolInner> = reader.read(&map_key::<Twox64Concat>(
				b"NominationPools",
				b"BondedPools",
				&m.pool_id.encode(),
			))?;
			let pool = pool.ok_or(ProofError::Incomplete)?;
			let (pool_active, _) = direct(&reader, &pool_bonded_account(pools, m.pool_id))?;
			if pool.points == 0 {
				0
			} else {
				pezsp_runtime::helpers_128bit::multiply_by_rational_with_rounding(
					m.points,
					pool_active,
					pool.points,
					pezsp_runtime::Rounding::Down,
				)
				.unwrap_or(0)
			}
		},
	};

	Ok(HubStake { staked: direct_active.saturating_add(pooled), nominations, unlocking_chunks })
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::hub_fixture as fx;

	const POOLS: PalletId = PalletId(*b"py/nopls");

	fn nodes(n: &[&[u8]]) -> Vec<Vec<u8>> {
		n.iter().map(|x| x.to_vec()).collect()
	}

	#[test]
	fn the_relay_proof_reaches_the_hubs_state_root() {
		let root =
			hub_state_root(H256(fx::RELAY_ROOT), &nodes(fx::RELAY_PROOF), 1000).expect("verifies");
		assert_eq!(root, H256(fx::HUB_ROOT));
	}

	#[test]
	fn a_relay_proof_for_another_root_is_refused() {
		let mut wrong = fx::RELAY_ROOT;
		wrong[0] ^= 1;
		assert!(hub_state_root(H256(wrong), &nodes(fx::RELAY_PROOF), 1000).is_err());
	}

	#[test]
	fn a_relay_proof_for_another_chain_does_not_reach_the_hub() {
		// The proof covers para 1000 only; anything else is not proven absent, it is unproven.
		assert!(hub_state_root(H256(fx::RELAY_ROOT), &nodes(fx::RELAY_PROOF), 1004).is_err());
	}

	#[test]
	fn a_direct_bond_reads_as_the_hub_has_it() {
		let stake = hub_stake(H256(fx::HUB_ROOT), &nodes(fx::STAKER_PROOF), &fx::STAKER, POOLS)
			.expect("verifies");
		assert_eq!(
			stake,
			HubStake { staked: fx::STAKER_ACTIVE, nominations: 0, unlocking_chunks: 0 }
		);
	}

	#[test]
	fn a_pool_members_share_reads_as_the_hub_has_it() {
		let stake = hub_stake(H256(fx::HUB_ROOT), &nodes(fx::MEMBER_PROOF), &fx::MEMBER, POOLS)
			.expect("verifies");
		assert_eq!(stake.staked, fx::MEMBER_POINTS * fx::POOL_ACTIVE / fx::POOL_POINTS);
		assert!(stake.staked > 0);
	}

	#[test]
	fn the_pool_account_is_derived_as_the_hub_derives_it() {
		assert_eq!(pool_bonded_account(POOLS, fx::MEMBER_POOL), fx::POOL_ACCOUNT);
	}

	#[test]
	fn a_proof_that_leaves_out_a_key_proves_nothing() {
		// The member's proof without its last node: the pool's bond is no longer covered, and
		// a missing value is not read as zero.
		let mut cut = nodes(fx::MEMBER_PROOF);
		cut.truncate(cut.len() / 2);
		assert!(hub_stake(H256(fx::HUB_ROOT), &cut, &fx::MEMBER, POOLS).is_err());
	}

	#[test]
	fn a_proof_for_someone_else_says_nothing_about_them() {
		// The staker's proof does not cover the member's keys.
		assert!(
			hub_stake(H256(fx::HUB_ROOT), &nodes(fx::STAKER_PROOF), &fx::MEMBER, POOLS).is_err()
		);
	}

	// Branches the live fixture does not reach, on a state built here. The layouts are written
	// out field by field from the hub's types, trailing fields included, so the readers above
	// are checked against an encoding they did not produce.
	mod built {
		use super::*;
		use codec::Compact;

		const WHO: [u8; 32] = [7; 32];
		const OTHER: [u8; 32] = [8; 32];

		fn key<H: StorageHasher<Output = Vec<u8>>>(p: &[u8], i: &[u8], k: &[u8]) -> Vec<u8> {
			map_key::<H>(p, i, k)
		}

		fn ledger(stash: [u8; 32], active: u128, chunks: u32) -> Vec<u8> {
			let unlocking: Vec<(Compact<u128>, Compact<u32>)> =
				(0..chunks).map(|e| (Compact(1u128), Compact(e))).collect();
			let legacy_claimed_rewards: Vec<u32> = vec![1, 2];
			(stash, Compact(active + 5), Compact(active), unlocking, legacy_claimed_rewards)
				.encode()
		}

		fn state(entries: Vec<(Vec<u8>, Vec<u8>)>, keys: &[Vec<u8>]) -> (H256, Vec<Vec<u8>>) {
			use pezsp_state_machine::{prove_read, Backend, InMemoryBackend};
			let backend = InMemoryBackend::<BlakeTwo256>::from((
				vec![(None, entries.into_iter().map(|(k, v)| (k, Some(v))).collect())],
				pezsp_core::storage::StateVersion::V1,
			));
			let root = *backend.root();
			let proof = prove_read(backend, keys).expect("proves");
			(root, proof.into_iter_nodes().collect())
		}

		fn keys(who: &[u8; 32]) -> Vec<Vec<u8>> {
			vec![
				key::<Twox64Concat>(b"Staking", b"Bonded", who),
				key::<Blake2_128Concat>(b"Staking", b"Ledger", who),
				key::<Twox64Concat>(b"Staking", b"Nominators", who),
				key::<Twox64Concat>(b"NominationPools", b"PoolMembers", who),
			]
		}

		#[test]
		fn a_ledger_that_names_another_stash_is_not_this_accounts_stake() {
			let (root, proof) = state(
				vec![
					(key::<Twox64Concat>(b"Staking", b"Bonded", &WHO), WHO.encode()),
					(key::<Blake2_128Concat>(b"Staking", b"Ledger", &WHO), ledger(OTHER, 900, 0)),
				],
				&keys(&WHO),
			);
			assert_eq!(hub_stake(root, &proof, &WHO, POOLS).unwrap().staked, 0);
		}

		#[test]
		fn unlocking_chunks_and_nominations_are_counted() {
			let nominations = (vec![[1u8; 32], [2; 32], [3; 32]], 4u32, false).encode();
			let (root, proof) = state(
				vec![
					(key::<Twox64Concat>(b"Staking", b"Bonded", &WHO), WHO.encode()),
					(key::<Blake2_128Concat>(b"Staking", b"Ledger", &WHO), ledger(WHO, 900, 2)),
					(key::<Twox64Concat>(b"Staking", b"Nominators", &WHO), nominations),
				],
				&keys(&WHO),
			);
			assert_eq!(
				hub_stake(root, &proof, &WHO, POOLS).unwrap(),
				HubStake { staked: 900, nominations: 3, unlocking_chunks: 2 }
			);
		}

		fn pool(points: u128, with_commission: bool) -> Vec<u8> {
			let current: Option<(u32, [u8; 32])> = with_commission.then_some((50_000_000, OTHER));
			let max: Option<u32> = with_commission.then_some(100_000_000);
			let change_rate: Option<(u32, u32)> = with_commission.then_some((1_000_000, 10));
			let throttle_from: Option<u32> = with_commission.then_some(12);
			// `CommissionClaimPermission::Account`
			let claim: Option<(u8, [u8; 32])> = with_commission.then_some((1, OTHER));
			let roles = (OTHER, Some(OTHER), Some(OTHER), Some(OTHER));
			let state = 0u8; // `PoolState::Open`
			(current, max, change_rate, throttle_from, claim, 3u32, points, roles, state).encode()
		}

		fn member_state(pool_points: u128, with_commission: bool) -> (H256, Vec<Vec<u8>>) {
			let account = pool_bonded_account(POOLS, 4);
			let member = (4u32, 300u128, 0u128, Vec::<(u32, u128)>::new()).encode();
			let mut ks = keys(&WHO);
			ks.push(key::<Twox64Concat>(b"NominationPools", b"BondedPools", &4u32.encode()));
			ks.push(key::<Twox64Concat>(b"Staking", b"Bonded", &account));
			ks.push(key::<Blake2_128Concat>(b"Staking", b"Ledger", &account));
			state(
				vec![
					(key::<Twox64Concat>(b"NominationPools", b"PoolMembers", &WHO), member),
					(
						key::<Twox64Concat>(b"NominationPools", b"BondedPools", &4u32.encode()),
						pool(pool_points, with_commission),
					),
					(key::<Twox64Concat>(b"Staking", b"Bonded", &account), account.encode()),
					(
						key::<Blake2_128Concat>(b"Staking", b"Ledger", &account),
						ledger(account, 2_000, 0),
					),
				],
				&ks,
			)
		}

		#[test]
		fn a_pools_commission_is_read_through_to_its_points() {
			let (root, proof) = member_state(600, true);
			// 300 of 600 points, of a 2 000 bond.
			assert_eq!(hub_stake(root, &proof, &WHO, POOLS).unwrap().staked, 1_000);
		}

		#[test]
		fn a_pool_without_points_gives_its_members_nothing() {
			let (root, proof) = member_state(0, false);
			assert_eq!(hub_stake(root, &proof, &WHO, POOLS).unwrap().staked, 0);
		}
	}
}
