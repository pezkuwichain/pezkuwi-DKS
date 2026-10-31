//! The names the state starts with. One list for the genesis of every chain and for the seed
//! migration of a chain already running, so the two cannot drift.
//!
//! The words are the tree's own spellings (tiki role names, the design's Kurmanji list, which
//! Serok confirmed on 2026-10-29). Office assignment is the runtime's, because the office type is.

/// Top-level names owned by Parliament: the state and the zone roots under which institutions
/// receive names (`zanko.edu.kurd`).
pub const ZONES: &[&[u8]] = &[b"gov", b"edu", b"mil", b"int", b"org", b"net"];

/// Names under `gov.kurd` granted at genesis.
pub const GOV_SUBNAMES: &[&[u8]] = &[
	b"serok",
	b"meclis",
	b"diwan",
	b"wezaret",
	b"darayi",
	b"parez",
	b"dad",
	b"belaw",
	b"tend",
	b"ava",
	b"cand",
	b"perwerde",
	b"xezine",
	b"bac",
	b"dks",
	b"hilbijartin",
	b"noter",
	b"balyoz",
];

/// What a locked label is, for explorers.
pub mod category {
	pub const INSTITUTION: u8 = 2;
	pub const NATION: u8 = 3;
	pub const CITY: u8 = 4;
	pub const TITLE: u8 = 5;
	pub const SAFETY: u8 = 6;
	/// The founder's own names, kept for the founder (Serok, 2026-10-30).
	pub const FOUNDER: u8 = 7;
}

/// Top-level labels nobody may buy; Parliament grants them.
pub const LOCKED: &[(&[u8], u8)] = &[
	// The founder's names: never sold, granted to the founder (Serok, 2026-10-30).
	(b"satoshiqazi", category::FOUNDER),
	(b"sqm", category::FOUNDER),
	// Institutions and products. `dks.kurd` is the news site's name (Serok, 2026-10-29).
	(b"dks", category::INSTITUTION),
	(b"pezkuwi", category::INSTITUTION),
	(b"pezkuwichain", category::INSTITUTION),
	(b"hez", category::INSTITUTION),
	(b"pez", category::INSTITUTION),
	(b"welati", category::INSTITUTION),
	(b"zagros", category::INSTITUTION),
	(b"rewshenbir", category::INSTITUTION),
	(b"pex", category::INSTITUTION),
	(b"bereketli", category::INSTITUTION),
	(b"kurdishtts", category::INSTITUTION),
	(b"whatskurd", category::INSTITUTION),
	// The nation.
	(b"kurd", category::NATION),
	(b"kurdistan", category::NATION),
	(b"newroz", category::NATION),
	(b"bakur", category::NATION),
	(b"bashur", category::NATION),
	(b"rojava", category::NATION),
	(b"rojhilat", category::NATION),
	// Cities, for their municipalities. The full list is its own piece of work.
	(b"amed", category::CITY),
	(b"hewler", category::CITY),
	(b"silemani", category::CITY),
	(b"kerkuk", category::CITY),
	(b"qamishlo", category::CITY),
	(b"kobani", category::CITY),
	(b"efrin", category::CITY),
	(b"mehabad", category::CITY),
	(b"wan", category::CITY),
	(b"dersim", category::CITY),
	// Offices, so nobody buys `serok.kurd` and passes for the Serok.
	(b"serok", category::TITLE),
	(b"wezir", category::TITLE),
	(b"parlementer", category::TITLE),
	(b"dadger", category::TITLE),
	(b"dozger", category::TITLE),
	(b"noter", category::TITLE),
	(b"xezinedar", category::TITLE),
	(b"bacgir", category::TITLE),
	(b"balyoz", category::TITLE),
	// Names a phishing page would want.
	(b"www", category::SAFETY),
	(b"mail", category::SAFETY),
	(b"api", category::SAFETY),
	(b"rpc", category::SAFETY),
	(b"wallet", category::SAFETY),
	(b"admin", category::SAFETY),
	(b"root", category::SAFETY),
	(b"support", category::SAFETY),
	(b"help", category::SAFETY),
	(b"login", category::SAFETY),
	(b"secure", category::SAFETY),
	(b"official", category::SAFETY),
	(b"bank", category::SAFETY),
	(b"pay", category::SAFETY),
	(b"airdrop", category::SAFETY),
	(b"govkurd", category::SAFETY),
	(b"gov-kurd", category::SAFETY),
	(b"g0v", category::SAFETY),
	(b"hikumet", category::SAFETY),
];

#[cfg(test)]
mod tests {
	use super::*;
	use crate::label::{validate, Use};

	#[test]
	fn every_genesis_label_is_valid_and_listed_once() {
		let mut seen = alloc::collections::BTreeSet::new();
		for l in ZONES.iter().chain(LOCKED.iter().map(|(l, _)| l)) {
			assert!(validate(l, Use::Grant).is_ok(), "{:?}", core::str::from_utf8(l));
			assert!(seen.insert(*l), "twice: {:?}", core::str::from_utf8(l));
		}
		for l in GOV_SUBNAMES {
			assert!(validate(l, Use::Sub).is_ok());
		}
	}
}
