//! Which content the register has stopped, computed from a snapshot of it.
//!
//! The rules are the pallet's, read from the outside:
//! - a name is hidden while it, or a name above it, holds a suspension for its own registration
//!   that has not run out (`Pezpallet::is_suspended`);
//! - a revoked name is deleted, so its content is gone from the register by the time anyone
//!   looks. What it served is known only from an earlier snapshot, which is why [`Seen`] is
//!   kept between runs, and once a revocation is seen it is kept for good.

use std::collections::{BTreeMap, BTreeSet};

use pezpallet_navnisan::{
	genesis_set::category, label::MAX_DEPTH, Content, Kind, Moment, NameHash, NameRecord,
	RegistrationId, Suspension,
};
use pezsp_core::crypto::AccountId32;
use serde::{Deserialize, Serialize};

pub type Record = NameRecord<AccountId32, pezpallet_tiki::Tiki, u128>;

/// The parts of the register this tool reads, all at one finalized block.
#[derive(Default)]
pub struct Snapshot {
	pub names: BTreeMap<NameHash, Record>,
	pub suspensions: BTreeMap<NameHash, Suspension>,
	pub locked: BTreeMap<NameHash, u8>,
	/// `Timestamp::Now` at that block, the clock the pallet itself measures suspensions by.
	pub now: Moment,
}

/// Something a gateway can be asked for by a name's content.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "cid")]
pub enum Root {
	/// A CID, binary.
	#[serde(with = "hex_bytes")]
	Ipfs(Vec<u8>),
	/// An IPNS name as a `libp2p-key` CID, binary.
	#[serde(with = "hex_bytes")]
	Ipns(Vec<u8>),
}

/// What a name served when it was last seen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeenName {
	pub parent: NameHash,
	pub sub: bool,
	pub roots: Vec<Root>,
	/// Set once the name is seen revoked; never cleared by this tool.
	#[serde(default)]
	pub revoked: bool,
}

/// Every name the previous runs saw, kept on disk between runs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
	pub names: BTreeMap<NameHash, SeenName>,
}

/// The roots a gateway would be asked for to serve `content`.
pub fn roots_of(content: &Content<AccountId32>) -> Vec<Root> {
	match content {
		Content::Ipfs(hash) => contenthash_root(hash).into_iter().collect(),
		Content::Feed { publisher, anchor, .. } => contenthash_root(anchor)
			.into_iter()
			.chain([ipns_of_ed25519(publisher)])
			.collect(),
	}
}

/// An EIP-1577 contenthash: `0xe3 0x01` + CID is IPFS, `0xe5 0x01` + CID is IPNS.
fn contenthash_root(hash: &[u8]) -> Option<Root> {
	match hash {
		[0xe3, 0x01, cid @ ..] if !cid.is_empty() => Some(Root::Ipfs(cid.to_vec())),
		[0xe5, 0x01, cid @ ..] if !cid.is_empty() => Some(Root::Ipns(cid.to_vec())),
		_ => None,
	}
}

/// The IPNS name of an ed25519 key: CIDv1, `libp2p-key`, identity multihash of the protobuf
/// `PublicKey { Type: Ed25519, Data: key }`.
pub fn ipns_of_ed25519(key: &[u8; 32]) -> Root {
	let mut cid = vec![0x01, 0x72, 0x00, 36, 0x08, 0x01, 0x12, 32];
	cid.extend_from_slice(key);
	Root::Ipns(cid)
}

fn suspension_holds(s: &Suspension, reg: RegistrationId, now: Moment) -> bool {
	s.reg == reg && now < s.until
}

/// `Pezpallet::is_suspended`, against the snapshot.
pub fn is_suspended(snap: &Snapshot, node: NameHash, rec: &Record) -> bool {
	let (mut node, mut rec) = (node, rec);
	for _ in 0..=MAX_DEPTH {
		if snap
			.suspensions
			.get(&node)
			.is_some_and(|s| suspension_holds(s, rec.reg, snap.now))
		{
			return true;
		}
		if rec.kind != Kind::Sub {
			return false;
		}
		node = rec.parent;
		match snap.names.get(&node) {
			Some(parent) => rec = parent,
			None => return false,
		}
	}
	false
}

/// Whether a name gone from the register went because it was revoked: it, or a name above it
/// that is also gone, has its label locked as revoked.
fn gone_by_revocation(snap: &Snapshot, seen: &Seen, node: NameHash) -> bool {
	let mut node = node;
	for _ in 0..=MAX_DEPTH {
		if snap.locked.get(&node) == Some(&category::REVOKED) {
			return true;
		}
		match seen.names.get(&node) {
			Some(s) if s.sub && !snap.names.contains_key(&s.parent) => node = s.parent,
			_ => return false,
		}
	}
	false
}

/// Brings `seen` up to `snap` and returns the content of every name that is suspended now or
/// was revoked. Content another name also serves is refused all the same: otherwise a stopped
/// site comes back the next minute under a second name pointing at the same CID.
pub fn decide(snap: &Snapshot, seen: &mut Seen) -> BTreeSet<Root> {
	let mut next = Seen::default();
	for (node, old) in &seen.names {
		if snap.names.contains_key(node) {
			continue;
		}
		if old.revoked || gone_by_revocation(snap, seen, *node) {
			next.names.insert(*node, SeenName { revoked: true, ..old.clone() });
		}
	}

	let mut refused = BTreeSet::new();
	for (node, rec) in &snap.names {
		let roots = rec.content.as_ref().map(roots_of).unwrap_or_default();
		if is_suspended(snap, *node, rec) {
			refused.extend(roots.iter().cloned());
		}
		next.names.insert(
			*node,
			SeenName { parent: rec.parent, sub: rec.kind == Kind::Sub, roots, revoked: false },
		);
	}
	for s in next.names.values().filter(|s| s.revoked) {
		refused.extend(s.roots.iter().cloned());
	}

	*seen = next;
	refused
}

mod hex_bytes {
	use serde::{Deserialize, Deserializer, Serializer};

	pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
		s.serialize_str(&hex::encode(bytes))
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
		hex::decode(String::deserialize(d)?).map_err(serde::de::Error::custom)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use pezpallet_navnisan::{Owner, SuspendedBy};
	use pezsp_core::H256;

	const NOW: Moment = 1_000_000;

	fn node(n: u8) -> NameHash {
		H256::repeat_byte(n)
	}

	fn cid(n: u8) -> Vec<u8> {
		// CIDv1 dag-pb sha2-256
		[vec![0x01, 0x70, 0x12, 0x20], vec![n; 32]].concat()
	}

	fn site(n: u8) -> Option<Content<AccountId32>> {
		Some(Content::Ipfs([vec![0xe3, 0x01], cid(n)].concat().try_into().unwrap()))
	}

	fn name(parent: NameHash, kind: Kind, reg: RegistrationId, content: u8) -> Record {
		NameRecord {
			label: b"satinal".to_vec().try_into().unwrap(),
			parent,
			parent_reg: 0,
			reg,
			owner: Owner::Account(AccountId32::new([1; 32])),
			kind,
			locked_transfer: false,
			expiry: None,
			account: None,
			content: site(content),
			deposit: None,
		}
	}

	fn suspension(reg: RegistrationId, until: Moment) -> Suspension {
		Suspension { until, reg, reason: H256::zero(), by: SuspendedBy::Court }
	}

	fn snapshot(names: &[(u8, Record)]) -> Snapshot {
		Snapshot {
			names: names.iter().map(|(n, r)| (node(*n), r.clone())).collect(),
			now: NOW,
			..Default::default()
		}
	}

	fn ipfs(n: u8) -> Root {
		Root::Ipfs(cid(n))
	}

	#[test]
	fn a_suspended_name_and_the_names_under_it_are_refused() {
		let mut snap = snapshot(&[
			(1, name(H256::zero(), Kind::Leased, 10, 1)),
			(2, name(node(1), Kind::Sub, 11, 2)),
			(3, name(H256::zero(), Kind::Leased, 12, 3)),
		]);
		let mut seen = Seen::default();
		assert!(decide(&snap, &mut seen).is_empty());

		snap.suspensions.insert(node(1), suspension(10, NOW + 1));
		assert_eq!(decide(&snap, &mut seen), BTreeSet::from([ipfs(1), ipfs(2)]));
	}

	#[test]
	fn a_suspension_stops_at_a_name_that_is_not_a_subname() {
		// A granted name under a zone root is not hidden by a suspension of the root, as in
		// `Pezpallet::is_suspended`
		let mut snap = snapshot(&[
			(1, name(H256::zero(), Kind::Granted, 10, 1)),
			(2, name(node(1), Kind::Granted, 11, 2)),
		]);
		snap.suspensions.insert(node(1), suspension(10, NOW + 1));
		assert_eq!(decide(&snap, &mut Seen::default()), BTreeSet::from([ipfs(1)]));
	}

	#[test]
	fn a_suspension_that_ran_out_or_names_another_registration_refuses_nothing() {
		let mut snap = snapshot(&[(1, name(H256::zero(), Kind::Leased, 10, 1))]);
		snap.suspensions.insert(node(1), suspension(10, NOW));
		assert!(decide(&snap, &mut Seen::default()).is_empty());

		snap.suspensions.insert(node(1), suspension(9, NOW + 1));
		assert!(decide(&snap, &mut Seen::default()).is_empty());
	}

	#[test]
	fn a_revoked_name_stays_refused_after_it_is_gone_and_its_label_unlocked() {
		let mut snap = snapshot(&[
			(1, name(H256::zero(), Kind::Leased, 10, 1)),
			(2, name(node(1), Kind::Sub, 11, 2)),
		]);
		let mut seen = Seen::default();
		assert!(decide(&snap, &mut seen).is_empty());

		// `enact_revocation`: the name and what is under it gone, the label locked as revoked
		snap.names.clear();
		snap.locked.insert(node(1), category::REVOKED);
		assert_eq!(decide(&snap, &mut seen), BTreeSet::from([ipfs(1), ipfs(2)]));

		// Parliament unlocks the label: the court's finding about the content still stands
		snap.locked.clear();
		assert_eq!(decide(&snap, &mut seen), BTreeSet::from([ipfs(1), ipfs(2)]));
	}

	#[test]
	fn a_name_that_lapsed_or_was_locked_for_another_reason_is_forgotten() {
		let mut snap = snapshot(&[(1, name(H256::zero(), Kind::Leased, 10, 1))]);
		let mut seen = Seen::default();
		decide(&snap, &mut seen);

		snap.names.clear();
		snap.locked.insert(node(1), category::INSTITUTION);
		assert!(decide(&snap, &mut seen).is_empty());
		assert!(seen.names.is_empty());
	}

	#[test]
	fn a_feed_refuses_its_anchor_and_its_publishers_ipns_name() {
		let publisher = [7u8; 32];
		let anchor = [vec![0xe3, 0x01], cid(4)].concat().try_into().unwrap();
		let feed = Content::Feed { publisher, anchorer: AccountId32::new([2; 32]), anchor, seq: 3 };
		assert_eq!(roots_of(&feed), vec![ipfs(4), ipns_of_ed25519(&publisher)]);

		let ipns = Content::Ipfs([vec![0xe5, 0x01], cid(5)].concat().try_into().unwrap());
		assert_eq!(roots_of(&ipns), vec![Root::Ipns(cid(5))]);
	}

	#[test]
	fn the_ipns_name_of_a_key_is_the_one_kubo_gives_it() {
		// NEW-4's public node key, and the name `ipfs key list -l --ipns-base=base36` shows for it
		let key: [u8; 32] =
			hex::decode("f179fe4ac65e4744503c2f56a370d432adbddaa11cd69e1b697ce1949d60368a")
				.unwrap()
				.try_into()
				.unwrap();
		let Root::Ipns(bytes) = ipns_of_ed25519(&key) else { panic!("not an IPNS root") };
		let name = cid::Cid::try_from(bytes.as_slice())
			.unwrap()
			.to_string_of_base(cid::multibase::Base::Base36Lower)
			.unwrap();
		assert_eq!(name, "k51qzi5uqu5dm798wknn6a2ikhdi8ptdehejav08liq955n5m31pe0y92bi6x6");
	}

	#[test]
	fn what_was_seen_survives_a_round_trip_to_disk() {
		let snap = snapshot(&[(1, name(H256::zero(), Kind::Leased, 10, 1))]);
		let mut seen = Seen::default();
		decide(&snap, &mut seen);
		let back: Seen = serde_json::from_slice(&serde_json::to_vec(&seen).unwrap()).unwrap();
		assert_eq!(back, seen);
	}
}
