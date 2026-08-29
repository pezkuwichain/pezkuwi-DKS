# TNPoS Phase 1 — Implementation Plan (M7.2 + M7.3)

> **For agent workers:** MANDATORY SUB-SKILL: implement this task by task with
> `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans`.
> The steps use checkbox (`- [ ]`) syntax so that progress can be tracked.

**Goal:** to build `pezpallet-tnpos`, which replaces `pezpallet-validator-pool`, selects a
27-member committee by stratified sampling from nine independent strata and enforces the
security budget as a runtime constraint — together with its mathematics/type base,
`pezkuwi-tnpos-primitives`.

**Architecture:** Two new crates. The primitives carry the stratum types, the shared score
traits, the `Sortition` boundary and the security mathematics — they are tested without a
runtime. The pallet manages the pool, eligibility, sampling and degradation; randomness sits
behind the `Sortition` trait, so that in Phase 2 ring-VRF is plugged in without touching the core.

**Tech Stack:** Rust, `no_std` + `std` dual target, PezFRAME (`pezframe_support::pezpallet`),
SCALE codec, `pezsp-io` blake2_256, `pezpallet-session`.

**Spec:** `docs/TNPOS_DESIGN.md` — the plan builds its argument from the spec; the two are read
together.

## Global Constraints

- **The Rust edition and the lints are inherited from the workspace.** `[lints] workspace = true`
  is mandatory in every new crate. `clippy::correctness = deny` — never suppressed.
- **`no_std` compatibility is mandatory.** Every piece of code that enters the runtime is built
  with `default-features = false`; `std` is for tests and analysis only. **Floating point
  (f32/f64) MUST NOT enter runtime code** — it is non-deterministic across platforms.
- **Every stored enum carries `#[codec(index = N)]`** and is pinned by a test that goes through
  `scale_info`. The pattern: the `stored_enum_encoding` module in
  `pezkuwi/pezpallets/validator-pool/src/types.rs` (copy it before that file is deleted).
- **The runtime is built with overflow checks off.** Every arithmetic operation must be
  `saturating_*` / `checked_*`; a bare `+` or `*` is rejected in code review.
- **Comments in English**, commit messages in English, PR descriptions in English.
- **`taplo` is always run with `--config .config/taplo.toml`.**
- **Heavy builds are not run on WSL** — `cargo test -p <crate>` is local and acceptable;
  runtime builds and benchmarks go to CI.
- Committee constants (spec §3): `k=9` strata × `3` seats = `n=27`, `q=19`,
  halt threshold `≥9`, fork threshold `≥11`.

## Scope — and what this plan leaves OUT

**In:** all of M7.2; and from M7.3 the pool, the strata, eligibility, the score cache,
sampling, the security constraint, degradation, committee export to the validating chain, and stratum-specific
slashing.

**Out, requiring a separate plan:**
- **M7.0 / M7.1** — the oracle and the People→Relay XCM channel. **M7.0 is blocking**: this plan
  can run on Zagros, but as long as the scores remain stubs it **cannot go to mainnet**.
- **M7.6 Phase 2** — ring-VRF sortition, a real SRS, **sub-rounds within an era and automatic
  resampling when finality stalls** (spec §7.2). The `Sortition` trait is designed to accept
  that addition without a rewrite.
- **M7.7 Phase 3** — the anonymous escrow bond, pseudonymous membership.
- **M7.4 / M7.5** — Monte Carlo and the formal spec; Task 4 produces their input.

---

## File Layout

**New crate: `pezkuwi/primitives/tnpos/`** → `pezkuwi-tnpos-primitives`

| File | Responsibility |
|---|---|
| `src/lib.rs` | Module declarations, re-exports |
| `src/stratum.rs` | `StratumId` (9 variants, pinned), `StratumConfig` |
| `src/committee.rs` | `quorum` / `halt_threshold` / `fork_threshold` — `const fn` |
| `src/scores.rs` | Shared score traits + `ScoreSnapshot` (staleness) — **closes P-1** |
| `src/sortition.rs` | The `Sortition` trait (the Phase 2 boundary) + `sample_k` (unbiased Fisher-Yates) |
| `src/invariant.rs` | The security constraint — **integer, `no_std`** |
| `src/analysis.rs` | Exact probability mathematics — **behind the `std` feature, never enters WASM** |

**New crate: `pezkuwi/pezpallets/tnpos/`** → `pezpallet-tnpos`

| File | Responsibility |
|---|---|
| `src/lib.rs` | Pallet declaration, Config, storage, calls, hooks, genesis |
| `src/pool.rs` | Join/leave, stratum eligibility |
| `src/scores.rs` | Score cache, fail-closed staleness |
| `src/sample.rs` | Stratified sampling, degradation |
| `src/slash.rs` | Stratum-specific penalty |
| `src/weights.rs`, `src/benchmarking.rs`, `src/mock.rs`, `src/tests.rs` | Standard |

**Deleted:** `pezkuwi/pezpallets/validator-pool/` (all of it) and its workspace/runtime entries.

---

## Task 1: Primitives crate skeleton + stratum types

**Files:**
- Create: `pezkuwi/primitives/tnpos/Cargo.toml`, `src/lib.rs`, `src/stratum.rs`
- Modify: `Cargo.toml` (workspace members ~line 565; workspace dependencies ~line 1250)

**Interfaces:**
- Produces: `StratumId` (Copy, Eq, Ord, codec-pinned), `StratumId::ALL: [StratumId; 9]`,
  `StratumConfig { id: StratumId, seats: u32, min_eligible: u32 }`

- [ ] **Step 1: Write the failing test** — at the end of `pezkuwi/primitives/tnpos/src/stratum.rs`:

```rust
#[cfg(test)]
mod tests {
	use super::*;
	use scale_info::{TypeDef, TypeInfo};

	#[test]
	fn all_lists_every_stratum_once() {
		let mut seen = StratumId::ALL.to_vec();
		seen.sort();
		seen.dedup();
		assert_eq!(seen.len(), 9, "StratumId::ALL must list all nine strata exactly once");
	}

	// A variant's index is what the chain wrote into storage; move it and old bytes decode
	// as a different stratum -- no error, a different answer.
	#[test]
	fn stratum_indices_are_pinned() {
		let info = <StratumId as TypeInfo>::type_info();
		let TypeDef::Variant(v) = info.type_def() else { panic!("StratumId is not an enum") };
		let got: Vec<(String, u8)> =
			v.variants().iter().map(|x| (x.name.to_string(), x.index)).collect();
		let want = [
			("Stake", 0u8), ("Meclis", 1), ("Divan", 2), ("Perwerde", 3), ("Tiki", 4),
			("WelatiLottery", 5), ("Geography", 6), ("Tenure", 7), ("Infrastructure", 8),
		];
		assert_eq!(got.len(), want.len(), "a new stratum needs its own index and a line here");
		for (i, (n, ix)) in want.iter().enumerate() {
			assert_eq!((got[i].0.as_str(), got[i].1), (*n, *ix), "stratum {i} moved");
		}
	}
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezkuwi-tnpos-primitives`
Expected: a compile error — `StratumId` is not defined.

- [ ] **Step 3: Write the minimal implementation**

`Cargo.toml` (crate):
```toml
[package]
name = "pezkuwi-tnpos-primitives"
version = "1.0.0"
description = "Shared types, score traits and security arithmetic for TNPoS"
authors.workspace = true
homepage.workspace = true
edition.workspace = true
license.workspace = true
publish = false
repository.workspace = true
documentation.workspace = true

[lints]
workspace = true

[dependencies]
codec = { workspace = true, default-features = false, features = ["derive", "max-encoded-len"] }
scale-info = { workspace = true, default-features = false, features = ["derive"] }
pezsp-io = { workspace = true, default-features = false }
pezsp-std = { workspace = true, default-features = false }

[features]
default = ["std"]
std = ["codec/std", "pezsp-io/std", "pezsp-std/std", "scale-info/std"]
```

`src/lib.rs`:
```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Shared primitives for TNPoS: strata, committee arithmetic, score traits and the
//! security invariant. Deliberately runtime-free so the arithmetic can be tested and
//! audited on its own.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod committee;
pub mod invariant;
pub mod scores;
pub mod sortition;
pub mod stratum;

#[cfg(feature = "std")]
pub mod analysis;

pub use stratum::{StratumConfig, StratumId};
```

`src/stratum.rs` (above the test module):
```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The nine strata a TNPoS committee is drawn from.
//!
//! Each stratum's gate belongs to a different source of authority. That is the hidden
//! condition of the security arithmetic: two strata gated by the same institution count
//! as one stratum, not two.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// One of the nine independent gates a committee seat can be drawn through.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
	Debug, TypeInfo, MaxEncodedLen, serde::Serialize, serde::Deserialize,
)]
pub enum StratumId {
	/// Bonded HEZ; ranked internally by the existing Phragmen election on Asset Hub.
	#[codec(index = 0)]
	Stake,
	/// Elected members of the assembly.
	#[codec(index = 1)]
	Meclis,
	/// Members of the court.
	#[codec(index = 2)]
	Divan,
	/// Holders of accredited education credentials.
	#[codec(index = 3)]
	Perwerde,
	/// Community-granted tikis. Office tikis are excluded on purpose: they would tie this
	/// stratum back to the assembly and collapse two gates into one.
	#[codec(index = 4)]
	Tiki,
	/// Drawn by lot from every citizen; gated only by citizenship.
	#[codec(index = 5)]
	WelatiLottery,
	/// Attested residence outside the region.
	#[codec(index = 6)]
	Geography,
	/// Uninterrupted, offence-free pool membership. Time cannot be bought or granted.
	#[codec(index = 7)]
	Tenure,
	/// Measured operating record on attested, independent infrastructure.
	#[codec(index = 8)]
	Infrastructure,
}

impl StratumId {
	/// Every stratum, in index order.
	pub const ALL: [StratumId; 9] = [
		StratumId::Stake,
		StratumId::Meclis,
		StratumId::Divan,
		StratumId::Perwerde,
		StratumId::Tiki,
		StratumId::WelatiLottery,
		StratumId::Geography,
		StratumId::Tenure,
		StratumId::Infrastructure,
	];
}

/// How many seats a stratum carries and how many eligible members it needs to carry them.
///
/// `min_eligible` is the runtime's whole security check. The probability argument behind
/// the number lives in `analysis`, which never reaches the runtime.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Copy, PartialEq, Eq, Debug, TypeInfo,
	MaxEncodedLen, serde::Serialize, serde::Deserialize,
)]
pub struct StratumConfig {
	pub id: StratumId,
	pub seats: u32,
	pub min_eligible: u32,
}
```

Workspace `Cargo.toml`, in the members list (keep the alphabetical order):
```toml
	"pezkuwi/primitives/tnpos",
```
In the workspace dependencies:
```toml
pezkuwi-tnpos-primitives = { path = "pezkuwi/primitives/tnpos", version = "1.0.0", default-features = false }
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezkuwi-tnpos-primitives`
Expected: 2 tests PASS.

- [ ] **Step 5: Format and commit**

```bash
cargo fmt -p pezkuwi-tnpos-primitives
taplo format --config .config/taplo.toml pezkuwi/primitives/tnpos/Cargo.toml Cargo.toml
git add pezkuwi/primitives/tnpos Cargo.toml
git commit -m "Add TNPoS stratum types with pinned codec indices"
```

---

## Task 2: Committee arithmetic

**Files:**
- Create: `pezkuwi/primitives/tnpos/src/committee.rs`

**Interfaces:**
- Consumes: nothing
- Produces: `const fn quorum(n: u32) -> u32`, `const fn halt_threshold(n: u32) -> u32`,
  `const fn fork_threshold(n: u32) -> u32`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn thresholds_match_the_specified_committee() {
		// Spec section 3: n=27 => q=19, halt at 9 seats, fork at 11.
		assert_eq!(quorum(27), 19);
		assert_eq!(halt_threshold(27), 9);
		assert_eq!(fork_threshold(27), 11);
	}

	#[test]
	fn one_third_of_the_strata_is_what_it_costs() {
		// Three powers of three seats each can stall a 27-seat committee but cannot fork it;
		// forking takes a fourth. This is the property the design is bought for.
		assert!(9 >= halt_threshold(27));
		assert!(9 < fork_threshold(27));
		assert!(12 >= fork_threshold(27));
	}

	#[test]
	fn the_thresholds_hold_at_the_top_of_the_domain() {
		// `Seating::quorum` calls these on a committee size read from storage, and the
		// runtime is built without overflow checks. A bare `2 * n` would wrap here and
		// return a quorum smaller than the committee it is supposed to bind.
		let n = u32::MAX;
		assert!(quorum(n) <= n);
		assert!(halt_threshold(n) >= 1);
		assert!(fork_threshold(n) >= halt_threshold(n));
	}

	#[test]
	fn quorum_never_exceeds_the_committee() {
		// A degraded committee still has to have a reachable quorum.
		for n in 1..=64u32 {
			assert!(quorum(n) <= n, "quorum({n}) = {} exceeds n", quorum(n));
			assert!(halt_threshold(n) >= 1);
		}
	}

	#[test]
	fn safety_margin_exceeds_liveness_margin_at_every_size_this_design_produces() {
		// Every stratum carries three seats, so a committee -- full or degraded -- is always
		// a multiple of three. Across that whole family the fork threshold stays strictly
		// above the halt threshold, which is what makes stalling the recoverable failure and
		// forking the one the budget is spent on.
		for n in (3..=64u32).step_by(3) {
			assert!(fork_threshold(n) > halt_threshold(n), "n = {n}");
		}
	}

	#[test]
	fn a_committee_of_size_one_mod_three_collapses_the_two_margins() {
		// Not a defect in the formulas -- a fact about them, written down so nobody "fixes"
		// it later. Where n leaves remainder one on division by three, q lands such that the
		// thresholds coincide: every set large enough to stall is large enough to fork, and
		// the safety margin is gone. Sizes of this shape must never be seated, which is a
		// condition `invariant::seat` enforces rather than something callers must remember.
		for n in (4..=64u32).step_by(3) {
			assert_eq!(fork_threshold(n), halt_threshold(n), "n = {n}");
		}
	}
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezkuwi-tnpos-primitives committee`
Expected: a compile error — `quorum` not found.

- [ ] **Step 3: Write the minimal implementation**

```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Committee arithmetic.
//!
//! GRANDPA finalises on more than two thirds of the voter weight, so the quorum is not a
//! parameter this chain gets to choose -- it is structural. What follows derives the two
//! thresholds that matter from it.

/// Votes needed to finalise: strictly more than two thirds of `n`.
///
/// The doubling happens in `u64`. These are called at runtime on a committee size read
/// from storage -- not only const-evaluated -- and the runtime is built without overflow
/// checks, so a bare `2 * n` would wrap silently and hand back a quorum smaller than the
/// committee it is meant to bind. `+ 1` cannot overflow, because `2n/3 < n` for every `n`.
pub const fn quorum(n: u32) -> u32 {
	(((2u64 * n as u64) / 3) as u32).saturating_add(1)
}

/// Seats an adversary needs to stop the committee reaching quorum.
///
/// Recoverable: a stalled committee is re-sampled. Compare `fork_threshold`, which is not.
pub const fn halt_threshold(n: u32) -> u32 {
	n.saturating_sub(quorum(n)).saturating_add(1)
}

/// Seats an adversary needs before two conflicting quorums can intersect in adversary-only
/// members -- that is, before the chain can fork. Not recoverable.
pub const fn fork_threshold(n: u32) -> u32 {
	(2u64 * quorum(n) as u64).saturating_sub(n as u64) as u32
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezkuwi-tnpos-primitives committee`
Expected: 10 tests PASS.

> **Note:** although these three functions are `const fn`, they are **also called at run time** —
> `Seating::quorum()` in Task 5 calls them with the live committee size read from storage.
> Because the runtime is built with overflow checks off, the multiplications are done in `u64`
> and the subtractions are `saturating_sub`; otherwise they would wrap silently.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p pezkuwi-tnpos-primitives
git add pezkuwi/primitives/tnpos/src/committee.rs pezkuwi/primitives/tnpos/src/lib.rs
git commit -m "Derive TNPoS halt and fork thresholds from the GRANDPA quorum"
```

---

## Task 3: Shared score traits + staleness (closes P-1)

**Files:**
- Create: `pezkuwi/primitives/tnpos/src/scores.rs`

**Interfaces:**
- Produces: `ScoreSnapshot<BlockNumber>`, `ScoreProvider<AccountId, BlockNumber>`
  (`trust_of`, `tiki_of`, `perwerde_of`, `referral_of`, `staking_of`)

**Why:** these traits are today **copied byte for byte** across the `perwerde` · `tiki` ·
`trust` · `referral` pallets (PLAN.md P-1). The single definition lives here.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_fresh_score_reads_its_value() {
		let s = ScoreSnapshot { value: 1_000u128, last_updated: 100u32 };
		assert_eq!(s.value_if_fresh(104, 4), Some(1_000));
	}

	#[test]
	fn a_stale_score_is_absent_not_old() {
		// The failure this guards against: a stalled cross-chain channel leaves the last
		// value in place and the chain keeps treating a months-old number as current.
		// Absent is the honest answer; the caller must then fail closed.
		let s = ScoreSnapshot { value: 1_000u128, last_updated: 100u32 };
		assert_eq!(s.value_if_fresh(105, 4), None);
	}

	#[test]
	fn a_score_from_the_future_is_treated_as_stale() {
		// Clock skew across chains must not silently extend freshness.
		let s = ScoreSnapshot { value: 1_000u128, last_updated: 200u32 };
		assert_eq!(s.value_if_fresh(100, 4), None);
	}
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezkuwi-tnpos-primitives scores`
Expected: a compile error — `ScoreSnapshot` not found.

- [ ] **Step 3: Write the minimal implementation**

```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Score providers and the freshness rule that governs them.
//!
//! These traits were duplicated byte for byte across four pallets. One definition lives
//! here; the pallets consume it.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// A score together with when it was last written.
///
/// Scores originate on the People chain and reach the relay chain over XCM. A channel can
/// stall, so a cached score carries its age and is read through `value_if_fresh`.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Copy, PartialEq, Eq, Debug, TypeInfo,
	MaxEncodedLen, Default,
)]
pub struct ScoreSnapshot<BlockNumber> {
	pub value: u128,
	pub last_updated: BlockNumber,
}

impl<BlockNumber: Copy + PartialOrd + core::ops::Sub<Output = BlockNumber>>
	ScoreSnapshot<BlockNumber>
{
	/// The value, or `None` if it is older than `max_age` or dated in the future.
	///
	/// Returning the stale value would be the dangerous answer: eligibility computed from a
	/// number nobody is still maintaining. `None` forces the caller to fail closed.
	pub fn value_if_fresh(&self, now: BlockNumber, max_age: BlockNumber) -> Option<u128> {
		if self.last_updated > now {
			return None;
		}
		if now - self.last_updated > max_age {
			return None;
		}
		Some(self.value)
	}
}

/// Every score TNPoS reads about an account, from one place.
pub trait ScoreProvider<AccountId, BlockNumber> {
	fn trust_of(who: &AccountId) -> ScoreSnapshot<BlockNumber>;
	fn tiki_of(who: &AccountId) -> ScoreSnapshot<BlockNumber>;
	fn perwerde_of(who: &AccountId) -> ScoreSnapshot<BlockNumber>;
	fn referral_of(who: &AccountId) -> ScoreSnapshot<BlockNumber>;
	fn staking_of(who: &AccountId) -> ScoreSnapshot<BlockNumber>;
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezkuwi-tnpos-primitives scores`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p pezkuwi-tnpos-primitives
git add pezkuwi/primitives/tnpos/src/scores.rs pezkuwi/primitives/tnpos/src/lib.rs
git commit -m "Define TNPoS score traits once, with staleness that fails closed"
```

---

## Task 4: Sampling — unbiased Fisher-Yates + the `Sortition` boundary

**Files:**
- Create: `pezkuwi/primitives/tnpos/src/sortition.rs`

**Interfaces:**
- Consumes: `StratumId` (Task 1)
- Produces: `trait Sortition<AccountId>` (`select`), `fn sample_k<T: Clone>(candidates: &[T],
  k: u32, seed: &[u8; 32], domain: &[u8]) -> alloc::vec::Vec<T>`

**Why this boundary:** in Phase 2 ring-VRF arrives as a second implementation of this trait;
the core pallet does not change.

**Why a new shuffle:** the deleted `validator-pool` read **a single byte** from the seed and
took `% (i+1)` — both a serious modulo bias and a collapse for pools larger than 256. Here the
index is derived from 64 bits by Lemire's multiply-shift method: constant cost, negligible bias,
no rejection loop (so the weight stays deterministic).

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
	use super::*;

	const SEED: [u8; 32] = [7u8; 32];

	#[test]
	fn picks_exactly_k_distinct_candidates() {
		let pool: Vec<u32> = (0..50).collect();
		let got = sample_k(&pool, 3, &SEED, b"stake");
		assert_eq!(got.len(), 3);
		let mut d = got.clone();
		d.sort();
		d.dedup();
		assert_eq!(d.len(), 3, "a member must not be seated twice");
		assert!(got.iter().all(|x| pool.contains(x)));
	}

	#[test]
	fn takes_everyone_when_k_exceeds_the_pool() {
		let pool: Vec<u32> = (0..2).collect();
		assert_eq!(sample_k(&pool, 3, &SEED, b"stake").len(), 2);
	}

	#[test]
	fn is_deterministic_for_the_same_seed_and_domain() {
		let pool: Vec<u32> = (0..50).collect();
		assert_eq!(sample_k(&pool, 3, &SEED, b"stake"), sample_k(&pool, 3, &SEED, b"stake"));
	}

	#[test]
	fn different_strata_do_not_share_a_draw() {
		// Without domain separation every stratum would seat the same positions of its list.
		let pool: Vec<u32> = (0..200).collect();
		assert_ne!(sample_k(&pool, 3, &SEED, b"stake"), sample_k(&pool, 3, &SEED, b"tiki"));
	}

	#[test]
	fn each_round_draws_from_its_own_randomness() {
		// If `round` were dropped from the hash preimage, every round of one call would
		// share a random word. The shrinking bound would still hand back distinct picks, so
		// the draw would look valid and every other test in this file would still pass --
		// but consecutive picks would track each other almost exactly instead of being
		// independent, and a draw whose second seat is a function of its first is a weaker
		// draw than the security argument assumes. This measures the consequence: over many
		// seeds, two draws from a large pool land next to each other only rarely.
		let pool: Vec<u32> = (0..1_000).collect();
		let trials = 500u32;
		let mut adjacent = 0u32;
		for i in 0..trials {
			let mut seed = [0u8; 32];
			seed[..4].copy_from_slice(&i.to_le_bytes());
			let got = sample_k(&pool, 2, &seed, b"stake");
			if got[0].abs_diff(got[1]) <= 2 {
				adjacent += 1;
			}
		}
		// Independent rounds collide this closely about half a percent of the time; a shared
		// word puts it near certainty. The bar is deliberately loose so this cannot flake.
		assert!(adjacent < trials / 10, "{adjacent} of {trials} draws were adjacent");
	}

	#[test]
	fn every_candidate_is_reachable() {
		// The old implementation read one byte per swap, so with a pool above 256 the tail
		// was unreachable -- members could never be seated at all. This is that regression.
		let pool: Vec<u32> = (0..400).collect();
		let mut seen = alloc::collections::BTreeSet::new();
		for i in 0..2_000u32 {
			let mut seed = [0u8; 32];
			seed[..4].copy_from_slice(&i.to_le_bytes());
			seen.extend(sample_k(&pool, 3, &seed, b"stake"));
		}
		assert!(seen.iter().any(|&x| x >= 300), "high indices were never drawn");
	}
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezkuwi-tnpos-primitives sortition`
Expected: a compile error — `sample_k` not found.

- [ ] **Step 3: Write the minimal implementation**

```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Drawing a stratum's seats.
//!
//! `Sortition` is where the randomness comes from; `sample_k` is how it is spent. Phase 1
//! supplies a commit-reveal seed behind that trait and phase 2 replaces it with ring-VRF
//! tickets -- the split is what keeps that swap from touching the pallet.

use crate::stratum::StratumId;
use alloc::vec::Vec;

/// Source of a stratum's draw for an era.
pub trait Sortition<AccountId> {
	/// The members seated for `stratum` in `era`, or `None` if this era cannot be drawn --
	/// no seed yet, or not enough tickets. `None` degrades the committee (the stratum is
	/// left unseated); it never falls back to an order an adversary could have chosen.
	fn select(
		era: u32,
		stratum: StratumId,
		candidates: &[AccountId],
		k: u32,
	) -> Option<Vec<AccountId>>;
}

/// An index below `bound`, derived from 64 bits by Lemire's multiply-shift.
///
/// Constant cost and no rejection loop, so the weight stays deterministic. The residual
/// bias is below 2^-32 for any bound this chain will use.
fn index_below(bound: u32, word: u64) -> u32 {
	(((word as u128) * (bound as u128)) >> 64) as u32
}

/// `k` distinct members of `candidates`, drawn by a partial Fisher-Yates shuffle.
///
/// `domain` separates the draws so two strata of the same size do not seat the same
/// positions of their lists.
pub fn sample_k<T: Clone>(candidates: &[T], k: u32, seed: &[u8; 32], domain: &[u8]) -> Vec<T> {
	let n = candidates.len();
	let take = core::cmp::min(k as usize, n);
	if take == 0 {
		return Vec::new();
	}

	let mut idx: Vec<u32> = (0..n as u32).collect();
	let mut out = Vec::with_capacity(take);

	for round in 0..take {
		// One hash per draw, bound to seed, domain and round: no counter state to get wrong.
		let mut preimage = Vec::with_capacity(32 + domain.len() + 4);
		preimage.extend_from_slice(seed);
		preimage.extend_from_slice(domain);
		preimage.extend_from_slice(&(round as u32).to_le_bytes());
		let h = pezsp_io::hashing::blake2_256(&preimage);

		let word = u64::from_le_bytes(
			h[..8].try_into().expect("blake2_256 returns 32 bytes; 8 always fit; qed"),
		);
		let remaining = (n - round) as u32;
		let pick = round + index_below(remaining, word) as usize;

		idx.swap(round, pick);
		out.push(candidates[idx[round] as usize].clone());
	}

	out
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezkuwi-tnpos-primitives sortition`
Expected: 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p pezkuwi-tnpos-primitives
git add pezkuwi/primitives/tnpos/src/sortition.rs pezkuwi/primitives/tnpos/src/lib.rs
git commit -m "Draw stratum seats with an unbiased shuffle behind a Sortition boundary"
```

---

## Task 5: The security constraint — integer, enforced in the runtime

**Files:**
- Create: `pezkuwi/primitives/tnpos/src/invariant.rs`

**Interfaces:**
- Consumes: `StratumConfig` (Task 1), `committee::*` (Task 2)
- Produces: `enum InvariantError`, `struct Seating { pub seated: Vec<StratumConfig>, pub n: u32 }`,
  `fn seat(strata: &[StratumConfig], eligible: &[u32]) -> Result<Seating, InvariantError>`
- Constants: `MIN_STRATA: u32 = 5`, `MIN_COMMITTEE: u32 = 15`

**Spec clarification (a decision taken during implementation):** the exact probability
computation **does not enter the runtime** — floating point is non-deterministic and a
nine-stratum convolution does not fit in the block budget. The runtime enforces a **sufficient
condition**: every seated stratum meets its `min_eligible` floor. The proof that this floor
implies the budget is produced by `analysis` in Task 6.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
	use super::*;
	use crate::stratum::StratumId;

	fn nine() -> Vec<StratumConfig> {
		StratumId::ALL
			.iter()
			.map(|&id| StratumConfig { id, seats: 3, min_eligible: 50 })
			.collect()
	}

	#[test]
	fn a_full_house_seats_twenty_seven() {
		let s = seat(&nine(), &[200; 9]).expect("nine healthy strata must seat");
		assert_eq!(s.n, 27);
		assert_eq!(s.seated.len(), 9);
	}

	#[test]
	fn a_short_stratum_is_not_seated() {
		// 49 eligible against a floor of 50: the stratum stands down rather than being
		// seated at a size its draw cannot make safe.
		let mut e = [200u32; 9];
		e[4] = 49;
		let s = seat(&nine(), &e).expect("eight strata still clear the floors");
		assert_eq!(s.n, 24);
		assert!(!s.seated.iter().any(|c| c.id == StratumId::Tiki));
	}

	#[test]
	fn seats_are_never_redistributed() {
		// The forbidden repair: handing a short stratum's seats to the strata that are
		// populated concentrates exactly the power the design exists to split.
		let mut e = [200u32; 9];
		e[4] = 0;
		let s = seat(&nine(), &e).unwrap();
		assert!(s.seated.iter().all(|c| c.seats == 3), "no stratum may grow to absorb seats");
		assert_eq!(s.n, 24);
	}

	#[test]
	fn too_few_strata_is_refused_not_degraded() {
		let mut e = [200u32; 9];
		for slot in e.iter_mut().take(5) {
			*slot = 0;
		}
		assert_eq!(seat(&nine(), &e), Err(InvariantError::TooFewStrata));
	}

	#[test]
	fn four_healthy_strata_are_still_too_few_gates() {
		// Population is not the point: four fully-populated strata still mean one collusion
		// short of deciding the chain, so the count is refused on its own.
		let four: Vec<StratumConfig> = nine().into_iter().take(4).collect();
		assert_eq!(seat(&four, &[200; 4]), Err(InvariantError::TooFewStrata));
	}

	#[test]
	fn a_repeated_stratum_is_refused() {
		// Nine entries naming eight gates is not nine gates. The budget is computed from the
		// number of independent gates, so a duplicate would let a configuration claim an
		// independence it does not have -- and every probability downstream would be wrong.
		let mut dup = nine();
		dup[8].id = dup[0].id;
		assert_eq!(seat(&dup, &[200; 9]), Err(InvariantError::DuplicateStratum));
	}

	#[test]
	fn a_committee_too_large_to_store_is_refused() {
		// The pallet keeps the seated committee in a bounded vector. A configuration whose
		// seats exceed that bound clears every other check here and then fails at an era
		// boundary -- the one place a configuration must never be allowed to fail.
		let huge: Vec<StratumConfig> = StratumId::ALL
			.iter()
			.map(|&id| StratumConfig { id, seats: 10, min_eligible: 50 })
			.collect();
		assert_eq!(seat(&huge, &[200; 9]), Err(InvariantError::CommitteeTooLarge));
	}

	#[test]
	fn a_stratum_carrying_no_seats_is_refused() {
		// A zero-seat stratum would count towards MIN_STRATA while carrying nothing: nine
		// gates on paper, eight in the committee. The number of independent gates is the
		// quantity the entire security budget is computed from, so it has to mean seats.
		let mut with_empty = nine();
		with_empty[3].seats = 0;
		assert_eq!(seat(&with_empty, &[200; 9]), Err(InvariantError::EmptyStratum));
	}

	#[test]
	fn mismatched_input_lengths_are_refused() {
		assert_eq!(seat(&nine(), &[200; 8]), Err(InvariantError::LengthMismatch));
	}
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezkuwi-tnpos-primitives invariant`
Expected: a compile error — `seat` not found.

- [ ] **Step 3: Write the minimal implementation**

```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The security budget, as a condition the chain refuses to run without.
//!
//! The probability argument is in `analysis` and never reaches the runtime: floating point
//! is not deterministic across platforms and a nine-way convolution does not fit a block.
//! What the runtime enforces is the sufficient condition that argument establishes -- every
//! seated stratum meets its floor -- which is integer arithmetic and costs nothing.

use crate::{committee::quorum, stratum::StratumConfig};
use alloc::vec::Vec;

/// Fewer independent gates than this and one collusion decides the chain.
pub const MIN_STRATA: u32 = 5;

/// Below this the committee is too small for the thresholds to mean anything.
pub const MIN_COMMITTEE: u32 = 15;

/// The most seats a committee may carry. The pallet stores the seated committee in a
/// bounded vector of exactly this size, so a configuration above it would pass validation
/// and then fail at an era boundary -- which is the one moment a configuration must not be
/// allowed to fail.
pub const MAX_COMMITTEE: u32 = 64;

/// Why a configuration cannot be seated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InvariantError {
	/// Fewer than `MIN_STRATA` strata clear their floor.
	TooFewStrata,
	/// Fewer than `MIN_COMMITTEE` seats survive.
	CommitteeTooSmall,
	/// `strata` and `eligible` describe different numbers of strata.
	LengthMismatch,
	/// A stratum declares zero seats, which would let it be counted as independent while
	/// carrying nothing.
	EmptyStratum,
	/// The same stratum appears twice. Nine entries naming eight gates is eight gates, and
	/// the security budget is computed from that count.
	DuplicateStratum,
	/// More seats than the pallet can store for a committee.
	CommitteeTooLarge,
}

/// Which strata are seated this era, and how large the committee therefore is.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Seating {
	pub seated: Vec<StratumConfig>,
	pub n: u32,
}

impl Seating {
	/// Votes needed to finalise at this committee's actual size.
	pub fn quorum(&self) -> u32 {
		quorum(self.n)
	}
}

/// Seat every stratum that meets its floor; refuse the era if too little survives.
///
/// A stratum that falls short stands down. Its seats are *not* handed to the strata that
/// are populated -- that repair would concentrate exactly the power the strata exist to
/// split, so it is unavailable by construction rather than by policy.
pub fn seat(strata: &[StratumConfig], eligible: &[u32]) -> Result<Seating, InvariantError> {
	if strata.len() != eligible.len() {
		return Err(InvariantError::LengthMismatch);
	}
	for (i, a) in strata.iter().enumerate() {
		if strata.iter().skip(i.saturating_add(1)).any(|b| b.id == a.id) {
			return Err(InvariantError::DuplicateStratum);
		}
	}
	if strata.iter().any(|c| c.seats == 0) {
		return Err(InvariantError::EmptyStratum);
	}

	let mut seated = Vec::with_capacity(strata.len());
	let mut n = 0u32;
	for (cfg, &have) in strata.iter().zip(eligible.iter()) {
		if have >= cfg.min_eligible {
			n = n.saturating_add(cfg.seats);
			seated.push(*cfg);
		}
	}

	if (seated.len() as u32) < MIN_STRATA {
		return Err(InvariantError::TooFewStrata);
	}
	if n < MIN_COMMITTEE {
		return Err(InvariantError::CommitteeTooSmall);
	}
	if n > MAX_COMMITTEE {
		return Err(InvariantError::CommitteeTooLarge);
	}

	Ok(Seating { seated, n })
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezkuwi-tnpos-primitives invariant`
Expected: 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p pezkuwi-tnpos-primitives
git add pezkuwi/primitives/tnpos/src/invariant.rs pezkuwi/primitives/tnpos/src/lib.rs
git commit -m "Refuse to seat a committee that misses the security floors"
```

---

## Task 6: Probability analysis — behind `std`, produces the whitepaper table

**Files:**
- Create: `pezkuwi/primitives/tnpos/src/analysis.rs`
- Modify: `pezkuwi/primitives/tnpos/Cargo.toml` (no test dependency; `std` is enough)

**Interfaces:**
- Consumes: `StratumConfig`, `committee::*`
- Produces: `fn stratum_distribution(eligible: u32, adversary: u32, seats: u32) -> Vec<f64>`,
  `fn committee_distribution(per_stratum: &[Vec<f64>]) -> Vec<f64>`,
  `fn tail(dist: &[f64], from: u32) -> f64`

**This proves Task 5's floor and is the input to M7.4/M7.5.** The code sits under
`#[cfg(feature = "std")]` and **does not enter** WASM.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
	use super::*;
	use crate::{committee::*, stratum::StratumId};

	fn approx(a: f64, b: f64, tol: f64) {
		assert!((a - b).abs() <= tol, "{a} vs {b}");
	}

	#[test]
	fn a_stratum_distribution_is_a_distribution() {
		let d = stratum_distribution(200, 10, 3);
		approx(d.iter().sum::<f64>(), 1.0, 1e-12);
		assert_eq!(d.len(), 4, "0..=3 adversary seats");
	}

	#[test]
	fn a_captured_stratum_yields_all_its_seats() {
		let d = stratum_distribution(3, 3, 3);
		approx(d[3], 1.0, 1e-12);
	}

	#[test]
	fn a_clean_stratum_yields_none() {
		let d = stratum_distribution(200, 0, 3);
		approx(d[0], 1.0, 1e-12);
	}

	// The number the design is sold on. Spec section 5: one power fully captured plus a
	// five percent presence in each of the other eight puts a fork past sixty years.
	#[test]
	fn the_published_budget_reproduces() {
		let mut per = vec![stratum_distribution(3, 3, 3)];
		for _ in 0..8 {
			per.push(stratum_distribution(200, 10, 3));
		}
		let d = committee_distribution(&per);
		approx(d.iter().sum::<f64>(), 1.0, 1e-9);

		let p_halt = tail(&d, halt_threshold(27));
		let p_fork = tail(&d, fork_threshold(27));
		approx(p_halt, 8.79e-4, 1e-5);
		approx(p_fork, 1.15e-5, 1e-6);

		// Four eras a day; the interval the whitepaper quotes.
		let years = (1.0 / p_fork) / 4.0 / 365.25;
		assert!((55.0..65.0).contains(&years), "fork interval drifted: {years} years");
	}

	// This is what the runtime's `min_eligible` floor has to buy. Below fifty eligible
	// members a stratum holding ten adversaries loses all three seats far too often.
	#[test]
	fn the_floor_of_fifty_is_where_the_stratum_becomes_safe() {
		let below = stratum_distribution(20, 10, 3)[3];
		let at = stratum_distribution(50, 10, 3)[3];
		assert!(below > 1e-2, "a pool of twenty is not safe: {below}");
		assert!(at < 1e-2, "the floor must bring it under one percent: {at}");
	}

	#[test]
	fn stratification_bounds_a_concentrated_adversary_where_probability_alone_would_not() {
		// Ninety adversary members against a pool of eighteen hundred, drawn two ways.
		//
		// Spread evenly, the two draws are near-identical: same mean, and stratification
		// only trims the variance. That is not what strata are for, and a test comparing
		// that case would be measuring nothing.
		//
		// The difference shows when the adversary concentrates -- which is the realistic
		// attack, since capital can buy the stake stratum outright and cannot buy the other
		// eight. Drawn as one pool those ninety can reach any number of seats. Drawn by
		// stratum they are capped at that stratum's three, and the fork probability is not
		// merely small but exactly zero: a deterministic bound where the flat draw offers
		// only a probabilistic one. That substitution is the whole argument for strata.
		let mut per = vec![stratum_distribution(200, 0, 3); 9];
		per[0] = stratum_distribution(200, 90, 3);
		let concentrated = tail(&committee_distribution(&per), fork_threshold(27));
		let flat = tail(&stratum_distribution(1800, 90, 27), fork_threshold(27));

		assert_eq!(concentrated, 0.0, "three seats is a cap, not a likelihood");
		assert!(flat > 0.0, "the same adversary keeps a real chance in a flat draw: {flat}");
	}
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezkuwi-tnpos-primitives analysis`
Expected: a compile error — `stratum_distribution` not found.

- [ ] **Step 3: Write the minimal implementation**

```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! What the security floors are worth, in probability.
//!
//! Std-only on purpose. This is the argument behind `invariant`, not a thing the chain
//! computes: the runtime enforces the floors, and these functions are how we know the
//! floors are the right ones. They also generate the table in `docs/TNPOS_DESIGN.md`, so
//! the published numbers and the tested numbers cannot drift apart.

/// Binomial coefficient as f64. Exact for the ranges here (n below a few thousand, k=3).
fn choose(n: u32, k: u32) -> f64 {
	if k > n {
		return 0.0;
	}
	let mut acc = 1.0f64;
	for i in 0..k {
		acc = acc * ((n - i) as f64) / ((i + 1) as f64);
	}
	acc
}

/// How many of one stratum's `seats` an adversary holding `adversary` of its `eligible`
/// members takes: the hypergeometric distribution, indexed 0..=seats.
pub fn stratum_distribution(eligible: u32, adversary: u32, seats: u32) -> Vec<f64> {
	let total = choose(eligible, seats);
	(0..=seats)
		.map(|x| {
			if x > adversary || seats - x > eligible.saturating_sub(adversary) || total == 0.0 {
				0.0
			} else {
				choose(adversary, x) * choose(eligible - adversary, seats - x) / total
			}
		})
		.collect()
}

/// The committee-wide distribution: the convolution of the strata's distributions, since
/// the draws are independent once the strata are.
pub fn committee_distribution(per_stratum: &[Vec<f64>]) -> Vec<f64> {
	let mut acc = vec![1.0f64];
	for d in per_stratum {
		let mut next = vec![0.0f64; acc.len() + d.len() - 1];
		for (i, &pi) in acc.iter().enumerate() {
			if pi == 0.0 {
				continue;
			}
			for (j, &pj) in d.iter().enumerate() {
				next[i + j] += pi * pj;
			}
		}
		acc = next;
	}
	acc
}

/// Probability of `from` or more adversary seats.
pub fn tail(dist: &[f64], from: u32) -> f64 {
	dist.iter().skip(from as usize).sum()
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezkuwi-tnpos-primitives analysis`
Expected: 6 tests PASS. In particular `the_published_budget_reproduces` — that one is the test
of the table in spec §5.

- [ ] **Step 5: Verify the `no_std` build**

Run: `cargo build -p pezkuwi-tnpos-primitives --no-default-features`
Expected: SUCCESS. `analysis` must not have been compiled — no floating point entered the runtime.

- [ ] **Step 6: Commit**

```bash
cargo fmt -p pezkuwi-tnpos-primitives
git add pezkuwi/primitives/tnpos/src/analysis.rs pezkuwi/primitives/tnpos/src/lib.rs
git commit -m "Prove the security floors in std-only analysis, tested against the spec table"
```

---

## Task 7: Pallet skeleton — Config, storage, genesis

**Files:**
- Create: `pezkuwi/pezpallets/tnpos/Cargo.toml`, `src/lib.rs`, `src/mock.rs`, `src/tests.rs`,
  `src/weights.rs`
- Modify: `Cargo.toml` (workspace members + dependencies)

**Interfaces:**
- Consumes: all of `pezkuwi-tnpos-primitives`
- Produces: `pezpallet_tnpos::{Config, Pezpallet, Event, Error}`; the storage items
  `Strata`, `PoolMembers`, `StratumSize`, `CurrentEra`, `CurrentCommittee`

- [ ] **Step 1: Write the failing test** — `src/tests.rs`:

```rust
use crate::{mock::*, *};
use pezframe_support::assert_ok;

#[test]
fn genesis_installs_the_nine_strata() {
	new_test_ext().execute_with(|| {
		assert_eq!(Strata::<Test>::get().len(), 9);
		assert_eq!(CurrentEra::<Test>::get(), 0);
		assert!(CurrentCommittee::<Test>::get().is_empty());
	});
}

#[test]
fn genesis_refuses_a_configuration_that_cannot_be_seated() {
	// Four strata cannot clear MIN_STRATA, so building genesis with them must fail rather
	// than start a chain whose committee is below the security budget from block zero.
	assert!(std::panic::catch_unwind(|| { new_test_ext_with_strata(4) }).is_err());
}

#[test]
fn set_strata_requires_the_manager_origin() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tnpos::set_strata(RuntimeOrigin::root(), nine_strata()));
	});
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezpallet-tnpos`
Expected: the crate does not exist.

- [ ] **Step 3: Write the minimal implementation**

`Cargo.toml`:
```toml
[package]
name = "pezpallet-tnpos"
version = "1.0.0"
description = "Trust-enhanced Nominated Proof-of-Stake committee selection for PezkuwiChain"
authors.workspace = true
homepage.workspace = true
edition.workspace = true
license.workspace = true
publish = false
repository.workspace = true
documentation.workspace = true

[lints]
workspace = true

[dependencies]
codec = { workspace = true, default-features = false, features = ["derive", "max-encoded-len"] }
scale-info = { workspace = true, default-features = false, features = ["derive"] }
log = { workspace = true, default-features = false }
pezframe-benchmarking = { workspace = true, default-features = false, optional = true }
pezframe-support = { workspace = true, default-features = false }
pezframe-system = { workspace = true, default-features = false }
pezkuwi-tnpos-primitives = { workspace = true, default-features = false }
pezpallet-session = { workspace = true, default-features = false }
pezsp-io = { workspace = true, default-features = false }
pezsp-runtime = { workspace = true, default-features = false }
pezsp-std = { workspace = true, default-features = false }

[dev-dependencies]
pezpallet-balances = { workspace = true }
pezsp-core = { workspace = true }
pezsp-io = { workspace = true }

[features]
default = ["std"]
std = [
	"codec/std",
	"log/std",
	"pezframe-benchmarking?/std",
	"pezframe-support/std",
	"pezframe-system/std",
	"pezkuwi-tnpos-primitives/std",
	"pezpallet-balances/std",
	"pezpallet-session/std",
	"pezsp-io/std",
	"pezsp-runtime/std",
	"pezsp-std/std",
	"scale-info/std",
]
runtime-benchmarks = [
	"pezframe-benchmarking/runtime-benchmarks",
	"pezframe-support/runtime-benchmarks",
	"pezframe-system/runtime-benchmarks",
	"pezpallet-balances/runtime-benchmarks",
	"pezsp-runtime/runtime-benchmarks",
]
try-runtime = [
	"pezframe-support/try-runtime",
	"pezframe-system/try-runtime",
	"pezpallet-balances/try-runtime",
	"pezsp-runtime/try-runtime",
]
```

`src/lib.rs` (the core — the calls are added from Task 8 onwards):
```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! # TNPoS
//!
//! Selects a validator committee by drawing a fixed number of seats from each of nine
//! independent strata. Buying one stratum outright buys three seats of twenty-seven, which
//! is neither enough to stall the chain nor to fork it; that bound is the design.
//!
//! See `docs/TNPOS_DESIGN.md` for the threat model and the security budget.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pezpallet::*;
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use alloc::vec::Vec;
use pezframe_support::{pezpallet_prelude::*, traits::Get};
use pezframe_system::pezpallet_prelude::*;
use pezkuwi_tnpos_primitives::{
	invariant::{seat, InvariantError, Seating},
	scores::ScoreProvider,
	sortition::Sortition,
	StratumConfig, StratumId,
};

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::{weights::WeightInfo, *};

	/// First version this pallet has ever had on chain. Written down so a future migration
	/// can tell whether it has run.
	pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config<RuntimeEvent: From<Event<Self>>> {
		type WeightInfo: crate::weights::WeightInfo;

		/// Where an era's draw comes from. Phase 1 supplies a commit-reveal seed; phase 2
		/// replaces this with ring-VRF tickets without touching the rest of the pallet.
		type Sortition: Sortition<Self::AccountId>;

		/// Cached People-chain scores. Reads go through `ScoreSnapshot::value_if_fresh`.
		type Scores: ScoreProvider<Self::AccountId, BlockNumberFor<Self>>;

		/// May set strata and force an era.
		type ManagerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// How old a cached score may be before it counts as absent.
		#[pezpallet::constant]
		type MaxScoreAge: Get<BlockNumberFor<Self>>;

		/// Blocks per era.
		#[pezpallet::constant]
		type EraLength: Get<BlockNumberFor<Self>>;

		/// Upper bound on pool members. Bounds every iteration in this pallet.
		#[pezpallet::constant]
		type MaxPoolSize: Get<u32>;
	}

	/// The strata this chain draws from, and what each carries.
	#[pezpallet::storage]
	pub type Strata<T: Config> =
		StorageValue<_, BoundedVec<StratumConfig, ConstU32<16>>, ValueQuery>;

	/// Which stratum each pool member stands in. A member stands in exactly one: an account
	/// in two strata would correlate them, and the security arithmetic assumes they are not.
	#[pezpallet::storage]
	pub type PoolMembers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, StratumId, OptionQuery>;

	/// Eligible members per stratum. Kept as a counter so seating never has to iterate.
	#[pezpallet::storage]
	pub type StratumSize<T: Config> =
		StorageMap<_, Twox64Concat, StratumId, u32, ValueQuery>;

	#[pezpallet::storage]
	pub type CurrentEra<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pezpallet::storage]
	pub type EraStart<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// The committee seated for the current era, in stratum order.
	#[pezpallet::storage]
	pub type CurrentCommittee<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, ConstU32<64>>, ValueQuery>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A member joined `stratum`.
		Joined { who: T::AccountId, stratum: StratumId },
		/// A member left the pool.
		Left { who: T::AccountId },
		/// A committee was seated. `unseated` names the strata that stood down.
		CommitteeSeated { era: u32, size: u32, quorum: u32, unseated: Vec<StratumId> },
		/// No committee could be seated; the previous one stays.
		SeatingRefused { era: u32 },
		/// The strata configuration changed.
		StrataSet { count: u32 },
	}

	#[pezpallet::error]
	pub enum Error<T> {
		AlreadyInPool,
		NotInPool,
		PoolFull,
		/// The account does not meet this stratum's gate.
		NotEligible,
		/// A score this decision needs is missing or too old. Deliberately not the same as
		/// `NotEligible`: a stalled channel is an outage, not a judgement about the account.
		ScoreUnavailable,
		/// The strata configuration cannot be seated at all.
		UnseatableConfiguration,
	}
}
```

`src/lib.rs`, genesis and the configuration call (inside the pezpallet module):
```rust
	#[pezpallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub strata: Vec<StratumConfig>,
		pub members: Vec<(T::AccountId, StratumId)>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			let bounded: BoundedVec<StratumConfig, ConstU32<16>> = self
				.strata
				.clone()
				.try_into()
				.expect("genesis declares at most sixteen strata; qed");

			// Validate the configuration, not the population: the pool is legitimately
			// empty at genesis and fills before the first era. Pretending every stratum is
			// full checks exactly the config-level floors -- stratum count, total seats,
			// no stratum carrying zero. A chain that cannot ever be seated must fail to
			// build rather than start and discover it at the first era boundary.
			let as_if_full = alloc::vec![u32::MAX; bounded.len()];
			seat(&bounded, &as_if_full).expect("genesis strata must be seatable; qed");

			Strata::<T>::put(&bounded);
			for (who, stratum) in &self.members {
				PoolMembers::<T>::insert(who, stratum);
				StratumSize::<T>::mutate(stratum, |n| *n = n.saturating_add(1));
			}
			CurrentEra::<T>::put(0u32);
		}
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Replace the strata configuration.
		///
		/// Refused unless the new configuration could be seated, so the chain cannot be
		/// governed into a shape that is outside its own security budget.
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(T::WeightInfo::set_strata())]
		pub fn set_strata(origin: OriginFor<T>, strata: Vec<StratumConfig>) -> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;
			let bounded: BoundedVec<StratumConfig, ConstU32<16>> =
				strata.try_into().map_err(|_| Error::<T>::UnseatableConfiguration)?;
			let as_if_full = alloc::vec![u32::MAX; bounded.len()];
			seat(&bounded, &as_if_full).map_err(|_| Error::<T>::UnseatableConfiguration)?;

			let count = bounded.len() as u32;
			Strata::<T>::put(bounded);
			Self::deposit_event(Event::StrataSet { count });
			Ok(())
		}
	}
```

> Later tasks add to this `#[pezpallet::call]` block: 0-1 join/leave (Task 8), 2 `force_new_era`
> (Task 9), 3-4 the seed (Task 10), 5 `report_offence` (Task 11). The indices must be **fixed**,
> not follow the order of definition.

`src/weights.rs` (in full — replaced in Task 13 with measured values):
```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Weights for pezpallet-tnpos.
//!
//! The zero implementation exists so tests can run before the benchmark pass; it is not a
//! weight anyone may ship. Task 13 replaces it with measured values from CI.

use pezframe_support::weights::Weight;

pub trait WeightInfo {
	fn join() -> Weight;
	fn leave() -> Weight;
	fn set_strata() -> Weight;
	fn report_offence() -> Weight;
	fn commit_seed() -> Weight;
	fn reveal_seed() -> Weight;
	/// `p` is the pool size: seating iterates `PoolMembers` once per seated stratum, so the
	/// cost is linear in it and a constant here would be a lie the block budget pays for.
	fn seat_committee(p: u32) -> Weight;
}

impl WeightInfo for () {
	fn join() -> Weight {
		Weight::zero()
	}
	fn leave() -> Weight {
		Weight::zero()
	}
	fn set_strata() -> Weight {
		Weight::zero()
	}
	fn report_offence() -> Weight {
		Weight::zero()
	}
	fn commit_seed() -> Weight {
		Weight::zero()
	}
	fn reveal_seed() -> Weight {
		Weight::zero()
	}
	fn seat_committee(_p: u32) -> Weight {
		Weight::zero()
	}
}
```

`src/mock.rs` (in full — **the tests of every later task use these helpers**):
```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Test runtime for pezpallet-tnpos.

use crate as pezpallet_tnpos;
use core::cell::RefCell;
use pezframe_support::{construct_runtime, derive_impl, parameter_types, traits::ConstU32};
use pezkuwi_tnpos_primitives::{scores::ScoreSnapshot, StratumConfig, StratumId};
use pezsp_runtime::BuildStorage;
use std::collections::BTreeMap;

pub type AccountId = u64;
pub type BlockNumber = u64;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

construct_runtime!(
	pub enum Test {
		System: pezframe_system,
		Tnpos: pezpallet_tnpos,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = pezframe_system::mocking::MockBlock<Test>;
	type AccountId = AccountId;
	type Lookup = pezsp_runtime::traits::IdentityLookup<AccountId>;
}

parameter_types! {
	pub const MaxScoreAge: BlockNumber = 100;
	pub const EraLength: BlockNumber = 50;
	pub const MaxPoolSize: u32 = 2_000;
}

// Scores are set directly by tests. Nothing here reaches another chain: the real source is
// XCM from the People chain and is M7.1, which is why the runtime still runs on stubs.
thread_local! {
	static SCORES: RefCell<BTreeMap<(AccountId, u8), (u128, BlockNumber)>> =
		RefCell::new(BTreeMap::new());
}

const TRUST: u8 = 0;
const TIKI: u8 = 1;
const PERWERDE: u8 = 2;
const REFERRAL: u8 = 3;
const STAKING: u8 = 4;

fn put_score(who: AccountId, kind: u8, value: u128, at: BlockNumber) {
	SCORES.with(|s| s.borrow_mut().insert((who, kind), (value, at)));
}

fn read_score(who: &AccountId, kind: u8) -> ScoreSnapshot<BlockNumber> {
	SCORES
		.with(|s| s.borrow().get(&(*who, kind)).copied())
		.map(|(value, last_updated)| ScoreSnapshot { value, last_updated })
		.unwrap_or(ScoreSnapshot { value: 0, last_updated: System::block_number() })
}

pub struct MockScores;
impl pezkuwi_tnpos_primitives::scores::ScoreProvider<AccountId, BlockNumber> for MockScores {
	fn trust_of(who: &AccountId) -> ScoreSnapshot<BlockNumber> {
		read_score(who, TRUST)
	}
	fn tiki_of(who: &AccountId) -> ScoreSnapshot<BlockNumber> {
		read_score(who, TIKI)
	}
	fn perwerde_of(who: &AccountId) -> ScoreSnapshot<BlockNumber> {
		read_score(who, PERWERDE)
	}
	fn referral_of(who: &AccountId) -> ScoreSnapshot<BlockNumber> {
		read_score(who, REFERRAL)
	}
	fn staking_of(who: &AccountId) -> ScoreSnapshot<BlockNumber> {
		read_score(who, STAKING)
	}
}

impl pezpallet_tnpos::Config for Test {
	type WeightInfo = ();
	type Sortition = crate::seed::CommitRevealSortition<Test>;
	type Scores = MockScores;
	type ManagerOrigin = pezframe_system::EnsureRoot<AccountId>;
	type MaxScoreAge = MaxScoreAge;
	type EraLength = EraLength;
	type MaxPoolSize = MaxPoolSize;
}

/// The nine strata at their specified sizes, with a floor small enough for tests.
pub fn nine_strata() -> Vec<StratumConfig> {
	StratumId::ALL
		.iter()
		.map(|&id| StratumConfig { id, seats: 3, min_eligible: 5 })
		.collect()
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	new_test_ext_with_strata(9)
}

/// Build genesis with the first `n` strata. Fewer than five must panic in `build`.
pub fn new_test_ext_with_strata(n: usize) -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pezpallet_tnpos::GenesisConfig::<Test> {
		strata: nine_strata().into_iter().take(n).collect(),
		members: Vec::new(),
	}
	.assimilate_storage(&mut t)
	.unwrap();
	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn set_perwerde(who: AccountId, v: u128) {
	put_score(who, PERWERDE, v, System::block_number());
}

pub fn set_perwerde_at(who: AccountId, v: u128, at: BlockNumber) {
	put_score(who, PERWERDE, v, at);
}

pub fn set_tiki(who: AccountId, v: u128) {
	put_score(who, TIKI, v, System::block_number());
}

/// An account holding office tikis and nothing else.
///
/// `tiki_of` excludes office tikis, so it reads zero. That exclusion is what keeps the Tiki
/// stratum independent of Meclis; without it the two gates would collapse into one and the
/// security arithmetic would be describing a chain that does not exist.
pub fn set_office_tiki_only(who: AccountId) {
	put_score(who, TIKI, 0, System::block_number());
	put_score(who, TRUST, 1_000, System::block_number());
}

/// Put `per` eligible members into every stratum and contribute a seed.
pub fn fill_every_stratum(per: u32) {
	let mut who: AccountId = 100;
	for &s in StratumId::ALL.iter() {
		for _ in 0..per {
			for kind in [TRUST, TIKI, PERWERDE, STAKING] {
				put_score(who, kind, 1_000, System::block_number());
			}
			assert!(Tnpos::join(RuntimeOrigin::signed(who), s).is_ok());
			who += 1;
		}
	}
	seed_the_era();
}

pub fn empty_stratum(s: StratumId) {
	let members: Vec<AccountId> = pezpallet_tnpos::PoolMembers::<Test>::iter()
		.filter_map(|(w, st)| (st == s).then_some(w))
		.collect();
	for w in members {
		assert!(Tnpos::leave(RuntimeOrigin::signed(w)).is_ok());
	}
}

/// Run one commit-reveal round so a draw is possible.
pub fn seed_the_era() {
	let who = pezpallet_tnpos::PoolMembers::<Test>::iter()
		.next()
		.map(|(w, _)| w)
		.unwrap_or(ALICE);
	let pre = [42u8; 32];
	let _ = Tnpos::commit_seed(RuntimeOrigin::signed(who), pezsp_io::hashing::blake2_256(&pre));
	let _ = Tnpos::reveal_seed(RuntimeOrigin::signed(who), pre);
}

pub fn clear_seed() {
	pezpallet_tnpos::NextSeed::<Test>::kill();
}

pub fn run_to_block(n: BlockNumber) {
	use pezframe_support::traits::OnInitialize;
	while System::block_number() < n {
		let next = System::block_number() + 1;
		System::set_block_number(next);
		Tnpos::on_initialize(next);
	}
}

pub fn advance_eras(n: u32) {
	pezpallet_tnpos::CurrentEra::<Test>::mutate(|e| *e = e.saturating_add(n));
}
```

> **Note:** `mock.rs` refers to Task 10's `crate::seed::CommitRevealSortition` and to Task 8's
> `join`/`leave`. While Task 7 is being implemented these two lines are left commented out and
> uncommented in the task concerned; Task 7's own tests do not need them.

Workspace `Cargo.toml`:
```toml
	"pezkuwi/pezpallets/tnpos",
```
```toml
pezpallet-tnpos = { path = "pezkuwi/pezpallets/tnpos", version = "1.0.0", default-features = false }
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezpallet-tnpos`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p pezpallet-tnpos
taplo format --config .config/taplo.toml pezkuwi/pezpallets/tnpos/Cargo.toml Cargo.toml
git add pezkuwi/pezpallets/tnpos Cargo.toml
git commit -m "Add pezpallet-tnpos skeleton with strata storage and genesis"
```

---

## Task 8: Joining the pool — eligibility is measured, not declared

**Files:**
- Create: `pezkuwi/pezpallets/tnpos/src/pool.rs`
- Modify: `src/lib.rs` (call indices 0-1), `src/tests.rs`

**Interfaces:**
- Consumes: `Strata`, `PoolMembers`, `StratumSize`, `T::Scores`
- Produces: `Pezpallet::<T>::join(origin, stratum)`, `Pezpallet::<T>::leave(origin)`,
  `fn eligible_for(who: &T::AccountId, stratum: StratumId) -> Result<(), Error<T>>`

**This closes the deleted pallet's most serious hole:** the old `validate_category_requirements`
read `min_stake` **from an argument supplied by the caller**; nobody looked at the real balance.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn joining_measures_the_score_it_does_not_take_the_callers_word() {
	new_test_ext().execute_with(|| {
		// ALICE has no perwerde credential in the mock; nothing she can put in the call
		// should get her into that stratum.
		set_perwerde(ALICE, 0);
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde),
			Error::<Test>::NotEligible
		);
		set_perwerde(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_eq!(PoolMembers::<Test>::get(ALICE), Some(StratumId::Perwerde));
		assert_eq!(StratumSize::<Test>::get(StratumId::Perwerde), 1);
	});
}

#[test]
fn a_stale_score_blocks_joining_and_says_so() {
	new_test_ext().execute_with(|| {
		set_perwerde_at(ALICE, 500, 1);
		run_to_block(1 + MaxScoreAge::get() + 1);
		// Not NotEligible: the account may well qualify. The chain simply does not know.
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde),
			Error::<Test>::ScoreUnavailable
		);
	});
}

#[test]
fn a_member_stands_in_exactly_one_stratum() {
	new_test_ext().execute_with(|| {
		set_perwerde(ALICE, 500);
		set_tiki(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Tiki),
			Error::<Test>::AlreadyInPool
		);
	});
}

#[test]
fn leaving_decrements_the_stratum_it_left() {
	new_test_ext().execute_with(|| {
		set_perwerde(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_ok!(Tnpos::leave(RuntimeOrigin::signed(ALICE)));
		assert_eq!(StratumSize::<Test>::get(StratumId::Perwerde), 0);
		assert_eq!(PoolMembers::<Test>::get(ALICE), None);
	});
}

#[test]
fn office_tikis_do_not_open_the_tiki_stratum() {
	// Tiki and Meclis must stay independent gates. An office tiki is granted by the
	// assembly, so counting it here would quietly collapse two strata into one and the
	// security arithmetic would be measuring a chain that does not exist.
	new_test_ext().execute_with(|| {
		set_office_tiki_only(ALICE);
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Tiki),
			Error::<Test>::NotEligible
		);
	});
}

#[test]
fn the_pool_is_bounded() {
	new_test_ext().execute_with(|| {
		for i in 0..MaxPoolSize::get() {
			let who = 1_000 + i as u64;
			set_perwerde(who, 500);
			assert_ok!(Tnpos::join(RuntimeOrigin::signed(who), StratumId::Perwerde));
		}
		set_perwerde(9_999, 500);
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(9_999), StratumId::Perwerde),
			Error::<Test>::PoolFull
		);
	});
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezpallet-tnpos pool`
Expected: `join` not found.

- [ ] **Step 3: Write the minimal implementation** — `src/pool.rs`:

```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Joining a stratum.
//!
//! Every gate here is *measured*. The pallet this replaces read the applicant's own
//! declared stake out of the call arguments and compared it against a constant, which let
//! anyone enter the stake stratum for nothing.

use crate::*;

impl<T: Config> Pezpallet<T> {
	/// Whether `who` passes `stratum`'s gate right now.
	///
	/// `ScoreUnavailable` and `NotEligible` are distinct on purpose: the first says the
	/// chain cannot see the account's standing, the second says it has seen it and the
	/// answer is no. Collapsing them would let an outage read as a judgement.
	pub(crate) fn eligible_for(
		who: &T::AccountId,
		stratum: StratumId,
	) -> Result<(), Error<T>> {
		let now = pezframe_system::Pezpallet::<T>::block_number();
		let age = T::MaxScoreAge::get();
		let fresh = |s: pezkuwi_tnpos_primitives::scores::ScoreSnapshot<BlockNumberFor<T>>| {
			s.value_if_fresh(now, age).ok_or(Error::<T>::ScoreUnavailable)
		};

		match stratum {
			StratumId::Stake => {
				// Rank inside this stratum is Phragmen's job on Asset Hub; the gate here is
				// only that the account has a staking standing at all.
				ensure!(fresh(T::Scores::staking_of(who))? > 0, Error::<T>::NotEligible);
			},
			StratumId::Perwerde => {
				ensure!(fresh(T::Scores::perwerde_of(who))? > 0, Error::<T>::NotEligible);
			},
			StratumId::Tiki => {
				// Community tikis only. `tiki_of` must exclude the twelve office tikis;
				// including them would tie this stratum to Meclis.
				ensure!(fresh(T::Scores::tiki_of(who))? > 0, Error::<T>::NotEligible);
			},
			StratumId::Meclis
			| StratumId::Divan
			| StratumId::WelatiLottery
			| StratumId::Geography
			| StratumId::Tenure
			| StratumId::Infrastructure => {
				// These gates are attested by their own authorities and reach this chain as
				// trust standing until their dedicated channels land in M7.1.
				ensure!(fresh(T::Scores::trust_of(who))? > 0, Error::<T>::NotEligible);
			},
		}
		Ok(())
	}

	pub(crate) fn do_join(who: T::AccountId, stratum: StratumId) -> DispatchResult {
		ensure!(!PoolMembers::<T>::contains_key(&who), Error::<T>::AlreadyInPool);
		let size: u32 = StratumId::ALL
			.iter()
			.fold(0u32, |a, &s| a.saturating_add(StratumSize::<T>::get(s)));
		ensure!(size < T::MaxPoolSize::get(), Error::<T>::PoolFull);

		Self::eligible_for(&who, stratum)?;

		PoolMembers::<T>::insert(&who, stratum);
		StratumSize::<T>::mutate(stratum, |n| *n = n.saturating_add(1));
		Self::deposit_event(Event::Joined { who, stratum });
		Ok(())
	}

	pub(crate) fn do_leave(who: T::AccountId) -> DispatchResult {
		let stratum = PoolMembers::<T>::take(&who).ok_or(Error::<T>::NotInPool)?;
		StratumSize::<T>::mutate(stratum, |n| *n = n.saturating_sub(1));
		Self::deposit_event(Event::Left { who });
		Ok(())
	}
}
```

The calls, into `src/lib.rs`:
```rust
	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Join `stratum`. Every gate is measured against current scores.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::join())]
		pub fn join(origin: OriginFor<T>, stratum: StratumId) -> DispatchResult {
			Self::do_join(ensure_signed(origin)?, stratum)
		}

		/// Leave the pool.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::leave())]
		pub fn leave(origin: OriginFor<T>) -> DispatchResult {
			Self::do_leave(ensure_signed(origin)?)
		}
	}
```
and add `pub mod pool;` to `lib.rs`.

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezpallet-tnpos`
Expected: 9 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p pezpallet-tnpos
git add pezkuwi/pezpallets/tnpos/src
git commit -m "Gate pool entry on measured scores instead of caller-declared values"
```

---

## Task 9: Seating the committee — stratified sampling and degradation

**Files:**
- Create: `pezkuwi/pezpallets/tnpos/src/sample.rs`
- Modify: `src/lib.rs` (hook + call index 2), `src/tests.rs`

**Interfaces:**
- Consumes: `seat` (Task 5), `sample_k` / `Sortition` (Task 4), `PoolMembers`, `StratumSize`
- Produces: `fn do_seat_committee() -> Result<Seating, Error<T>>`,
  `Pezpallet::<T>::force_new_era(origin)`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_healthy_pool_seats_twenty_seven_across_nine_strata() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let c = CurrentCommittee::<Test>::get();
		assert_eq!(c.len(), 27);
		let mut per = std::collections::BTreeMap::new();
		for who in c.iter() {
			*per.entry(PoolMembers::<Test>::get(who).unwrap()).or_insert(0) += 1;
		}
		assert_eq!(per.len(), 9);
		assert!(per.values().all(|&v| v == 3), "each stratum seats exactly three");
	});
}

#[test]
fn a_short_stratum_shrinks_the_committee_it_does_not_hand_its_seats_away() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		empty_stratum(StratumId::Tiki);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let c = CurrentCommittee::<Test>::get();
		assert_eq!(c.len(), 24, "three seats are lost, not moved");
		assert!(!c.iter().any(|w| PoolMembers::<Test>::get(w) == Some(StratumId::Tiki)));
	});
}

#[test]
fn seating_is_refused_rather_than_run_below_the_budget() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		for s in [StratumId::Tiki, StratumId::Divan, StratumId::Geography,
			StratumId::Tenure, StratumId::Infrastructure] {
			empty_stratum(s);
		}
		let before = CurrentCommittee::<Test>::get();
		assert_noop!(
			Tnpos::force_new_era(RuntimeOrigin::root()),
			Error::<Test>::UnseatableConfiguration
		);
		assert_eq!(CurrentCommittee::<Test>::get(), before, "the old committee stays");
	});
}

#[test]
fn nobody_is_seated_twice() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let mut c = CurrentCommittee::<Test>::get().to_vec();
		let n = c.len();
		c.sort();
		c.dedup();
		assert_eq!(c.len(), n);
	});
}

#[test]
fn a_new_era_draws_a_different_committee() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(200);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let first = CurrentCommittee::<Test>::get();
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		assert_ne!(CurrentCommittee::<Test>::get(), first);
	});
}

#[test]
fn the_era_advances_on_schedule() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		run_to_block(EraLength::get());
		assert_eq!(CurrentEra::<Test>::get(), 1);
	});
}

#[test]
fn a_refused_seating_does_not_retry_every_block() {
	// The pallet this replaces swallowed the error and left EraStart untouched, so it
	// re-ran the whole selection on every single block and paid full weight for it.
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		for s in [StratumId::Tiki, StratumId::Divan, StratumId::Geography,
			StratumId::Tenure, StratumId::Infrastructure] {
			empty_stratum(s);
		}
		run_to_block(EraLength::get());
		// The window must have been written even though seating failed. Asserting only that
		// it is unchanged between two blocks would pass while it sat at genesis zero, which
		// is the exact bug this test exists for.
		assert_eq!(
			EraStart::<Test>::get(),
			EraLength::get(),
			"a failed seating must still move the era window"
		);
		run_to_block(EraLength::get() + 1);
		assert_eq!(
			EraStart::<Test>::get(),
			EraLength::get(),
			"and must not fire again on the very next block"
		);
	});
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezpallet-tnpos sample`
Expected: `force_new_era` not found.

- [ ] **Step 3: Write the minimal implementation** — `src/sample.rs`:

```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Seating a committee.

use crate::*;
use alloc::vec;

impl<T: Config> Pezpallet<T> {
	/// Candidates standing in `stratum`, bounded by `MaxPoolSize`.
	fn candidates(stratum: StratumId) -> Vec<T::AccountId> {
		PoolMembers::<T>::iter()
			.take(T::MaxPoolSize::get() as usize)
			.filter_map(|(who, s)| (s == stratum).then_some(who))
			.collect()
	}

	/// Draw a committee for the next era.
	///
	/// A stratum that misses its floor stands down and the committee is smaller for it.
	/// Its seats are never given to another stratum: that repair would concentrate the very
	/// power the strata exist to divide, so `seat` does not offer it.
	pub(crate) fn do_seat_committee() -> Result<Seating, Error<T>> {
		let strata = Strata::<T>::get();
		let sizes: Vec<u32> =
			strata.iter().map(|c| StratumSize::<T>::get(c.id)).collect();

		let seating = seat(&strata, &sizes).map_err(|e| match e {
			InvariantError::TooFewStrata
			| InvariantError::CommitteeTooSmall
			| InvariantError::LengthMismatch
			| InvariantError::EmptyStratum => Error::<T>::UnseatableConfiguration,
		})?;

		let era = CurrentEra::<T>::get().saturating_add(1);
		let mut committee = Vec::with_capacity(seating.n as usize);
		for cfg in seating.seated.iter() {
			let pool = Self::candidates(cfg.id);
			let drawn = T::Sortition::select(era, cfg.id, &pool, cfg.seats)
				.ok_or(Error::<T>::UnseatableConfiguration)?;
			committee.extend(drawn);
		}

		// The ceiling is named once, in the primitives crate, and `CurrentCommittee` is
		// declared against the same constant -- two places that must agree should not be two
		// numbers. `seat` already refuses configurations above it, so this conversion cannot
		// fail today; the branch stays because a future change to `seat` should surface as a
		// refused era rather than a panic.
		let bounded: BoundedVec<
			T::AccountId,
			ConstU32<{ pezkuwi_tnpos_primitives::invariant::MAX_COMMITTEE }>,
		> = committee.try_into().map_err(|_| Error::<T>::UnseatableConfiguration)?;

		let unseated: Vec<StratumId> = strata
			.iter()
			.filter(|c| !seating.seated.iter().any(|s| s.id == c.id))
			.map(|c| c.id)
			.collect();

		CurrentEra::<T>::put(era);
		EraStart::<T>::put(pezframe_system::Pezpallet::<T>::block_number());
		CurrentCommittee::<T>::put(&bounded);

		Self::deposit_event(Event::CommitteeSeated {
			era,
			size: seating.n,
			quorum: seating.quorum(),
			unseated,
		});
		Ok(seating)
	}
}
```

The hook and the call, into `src/lib.rs`:
```rust
	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let mut weight = T::DbWeight::get().reads(1);
			if now < EraStart::<T>::get().saturating_add(T::EraLength::get()) {
				return weight;
			}

			// The era window moves on whether or not a committee could be drawn. The pallet
			// this replaces left it in place on failure and so re-ran the entire selection
			// on every block, paying full weight each time and never recovering.
			EraStart::<T>::put(now);
			weight = weight.saturating_add(T::DbWeight::get().writes(1));
			weight = weight.saturating_add(T::WeightInfo::seat_committee(T::MaxPoolSize::get()));

			if Self::do_seat_committee().is_err() {
				Self::deposit_event(Event::SeatingRefused { era: CurrentEra::<T>::get() });
				log::warn!(target: "tnpos", "no committee could be seated; previous one stands");
			}
			weight
		}

		#[cfg(feature = "try-runtime")]
		fn try_state(_: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			let strata = Strata::<T>::get();
			let sizes: Vec<u32> = strata.iter().map(|c| StratumSize::<T>::get(c.id)).collect();
			// A live chain whose strata cannot be seated is a chain running outside its
			// security budget; that must surface as a failure, not as a quiet degradation.
			seat(&strata, &sizes).map_err(|_| "tnpos: strata cannot satisfy the security floors")?;
			Ok(())
		}
	}
```
```rust
		/// Seat a new committee now.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::seat_committee(T::MaxPoolSize::get()))]
		pub fn force_new_era(origin: OriginFor<T>) -> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;
			match Self::do_seat_committee() {
				Ok(_) => Ok(()),
				Err(e) => {
					Self::deposit_event(Event::SeatingRefused { era: CurrentEra::<T>::get() });
					Err(e.into())
				},
			}
		}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezpallet-tnpos`
Expected: 16 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p pezpallet-tnpos
git add pezkuwi/pezpallets/tnpos/src
git commit -m "Seat committees by stratum, shrinking rather than redistributing seats"
```

---

## Task 10: Phase 1 randomness — the commit-reveal seed

**Files:**
- Create: `pezkuwi/pezpallets/tnpos/src/seed.rs`
- Modify: `src/lib.rs` (call indices 3-4, storage), `src/tests.rs`

**Interfaces:**
- Produces: `struct CommitRevealSortition<T>` (`impl Sortition<T::AccountId>`),
  `Pezpallet::<T>::commit_seed(origin, hash)`, `Pezpallet::<T>::reveal_seed(origin, preimage)`

**Why commit-reveal:** `RandomnessFromOneEpochAgo` is **computable by anyone** an epoch in
advance, and the epoch's last block producers can bias it by withholding a block. For Phase 1 a
participatory commit-reveal is enough: if there is at least one honest participant the seed is
unpredictable. Phase 2 replaces this with ring-VRF.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_revealed_seed_matches_its_commitment() {
	new_test_ext().execute_with(|| {
		let pre = [3u8; 32];
		assert_ok!(Tnpos::commit_seed(RuntimeOrigin::signed(ALICE), blake2_256(&pre)));
		assert_ok!(Tnpos::reveal_seed(RuntimeOrigin::signed(ALICE), pre));
		assert!(NextSeed::<Test>::get().is_some());
	});
}

#[test]
fn a_reveal_that_does_not_match_is_rejected() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tnpos::commit_seed(RuntimeOrigin::signed(ALICE), blake2_256(&[3u8; 32])));
		assert_noop!(
			Tnpos::reveal_seed(RuntimeOrigin::signed(ALICE), [9u8; 32]),
			Error::<Test>::BadReveal
		);
	});
}

#[test]
fn one_honest_contributor_changes_the_seed() {
	// The property the whole scheme rests on: an adversary who reveals last still cannot
	// choose the result, because every contribution is mixed in.
	new_test_ext().execute_with(|| {
		assert_ok!(Tnpos::commit_seed(RuntimeOrigin::signed(ALICE), blake2_256(&[1u8; 32])));
		assert_ok!(Tnpos::reveal_seed(RuntimeOrigin::signed(ALICE), [1u8; 32]));
		let only_alice = NextSeed::<Test>::get().unwrap();
		assert_ok!(Tnpos::commit_seed(RuntimeOrigin::signed(BOB), blake2_256(&[2u8; 32])));
		assert_ok!(Tnpos::reveal_seed(RuntimeOrigin::signed(BOB), [2u8; 32]));
		assert_ne!(NextSeed::<Test>::get().unwrap(), only_alice);
	});
}

#[test]
fn seating_is_refused_when_no_seed_was_contributed() {
	// Falling back to a predictable seed would hand an adversary the draw. Refusing keeps
	// the previous committee, which is the safe direction.
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		clear_seed();
		assert_noop!(
			Tnpos::force_new_era(RuntimeOrigin::root()),
			Error::<Test>::UnseatableConfiguration
		);
	});
}

#[test]
fn only_pool_members_may_contribute() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tnpos::commit_seed(RuntimeOrigin::signed(9_999), blake2_256(&[1u8; 32])),
			Error::<Test>::NotInPool
		);
	});
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezpallet-tnpos seed`
Expected: `commit_seed` not found.

- [ ] **Step 3: Write the minimal implementation** — `src/seed.rs`:

```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Phase 1 randomness: a participatory commit-reveal.
//!
//! `RandomnessFromOneEpochAgo` is computable by everyone an epoch ahead and can be nudged
//! by whoever authors the epoch's last blocks -- both fatal for a draw whose whole value is
//! that nobody can see it coming. Commit-reveal is unpredictable as long as one contributor
//! is honest, which is enough to run on Zagros while ring-VRF lands in phase 2.

use crate::*;
use pezsp_io::hashing::blake2_256;

impl<T: Config> Pezpallet<T> {
	pub(crate) fn do_commit_seed(who: T::AccountId, hash: [u8; 32]) -> DispatchResult {
		ensure!(PoolMembers::<T>::contains_key(&who), Error::<T>::NotInPool);
		SeedCommitments::<T>::insert(&who, hash);
		Ok(())
	}

	pub(crate) fn do_reveal_seed(who: T::AccountId, preimage: [u8; 32]) -> DispatchResult {
		let commitment = SeedCommitments::<T>::take(&who).ok_or(Error::<T>::NoCommitment)?;
		ensure!(blake2_256(&preimage) == commitment, Error::<T>::BadReveal);

		// Mix rather than replace: a contributor who reveals last must not be able to pick
		// the outcome by choosing when to speak.
		NextSeed::<T>::mutate(|slot| {
			let mut buf = [0u8; 64];
			buf[..32].copy_from_slice(&slot.unwrap_or_default());
			buf[32..].copy_from_slice(&preimage);
			*slot = Some(blake2_256(&buf));
		});
		Ok(())
	}
}

/// Phase 1 `Sortition`: mixed commit-reveal seed, spent through `sample_k`.
pub struct CommitRevealSortition<T>(core::marker::PhantomData<T>);

impl<T: Config> pezkuwi_tnpos_primitives::sortition::Sortition<T::AccountId>
	for CommitRevealSortition<T>
{
	fn select(
		era: u32,
		stratum: StratumId,
		candidates: &[T::AccountId],
		k: u32,
	) -> Option<Vec<T::AccountId>> {
		// No contribution this era means no draw. Refusing degrades the committee; the
		// alternative -- a predictable fallback seed -- would hand the draw to whoever
		// stayed silent.
		let base = NextSeed::<T>::get()?;
		let mut buf = [0u8; 36];
		buf[..32].copy_from_slice(&base);
		buf[32..].copy_from_slice(&era.to_le_bytes());
		let seed = blake2_256(&buf);
		Some(pezkuwi_tnpos_primitives::sortition::sample_k(
			candidates,
			k,
			&seed,
			&[stratum as u8],
		))
	}
}
```

Storage and errors, into `src/lib.rs`:
```rust
	/// Commitments for the next era's seed.
	#[pezpallet::storage]
	pub type SeedCommitments<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, [u8; 32], OptionQuery>;

	/// The mixed seed for the next draw. `None` means no draw is possible.
	#[pezpallet::storage]
	pub type NextSeed<T: Config> = StorageValue<_, [u8; 32], OptionQuery>;
```
To `Error`: `NoCommitment`, `BadReveal`.
The calls (indices 3, 4): `commit_seed`, `reveal_seed`; add `pub mod seed;`.

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezpallet-tnpos`
Expected: 21 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p pezpallet-tnpos
git add pezkuwi/pezpallets/tnpos/src
git commit -m "Draw eras from a mixed commit-reveal seed instead of a predictable one"
```

---

## Task 11: Stratum-specific penalty

**Files:**
- Create: `pezkuwi/pezpallets/tnpos/src/slash.rs`
- Modify: `src/lib.rs` (storage, call index 5), `src/tests.rs`

**Interfaces:**
- Produces: `enum Offence { Unavailable, Equivocation }`,
  `Pezpallet::<T>::report_offence(origin, who, offence)`, the `Banned` storage item

**Design (spec §9):** in the stake stratum the penalty is the existing `staking-async` slashing,
and this pallet does not touch it. In the other eight strata what is punished is **standing**: a
pool ban (24 eras for minor, 360 eras for severe) and a trust penalty. The anonymous escrow bond
is Phase 3.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn an_offence_bans_the_member_from_the_pool() {
	new_test_ext().execute_with(|| {
		set_perwerde(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), ALICE, Offence::Equivocation));
		assert_eq!(PoolMembers::<Test>::get(ALICE), None);
		assert_eq!(StratumSize::<Test>::get(StratumId::Perwerde), 0);
		assert_eq!(Banned::<Test>::get(ALICE), Some(360));
	});
}

#[test]
fn a_banned_member_cannot_rejoin_until_the_ban_expires() {
	new_test_ext().execute_with(|| {
		set_perwerde(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), ALICE, Offence::Unavailable));
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde),
			Error::<Test>::Banned
		);
		advance_eras(24);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
	});
}

#[test]
fn being_unavailable_costs_less_than_equivocating() {
	// Going offline is a failure; signing two conflicting blocks is an attack. The ladder
	// has to say so, or the deterrent for the thing that can fork the chain is the same as
	// the deterrent for a bad connection.
	new_test_ext().execute_with(|| {
		set_perwerde(ALICE, 500);
		set_perwerde(BOB, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(BOB), StratumId::Perwerde));
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), ALICE, Offence::Unavailable));
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), BOB, Offence::Equivocation));
		assert!(Banned::<Test>::get(BOB) > Banned::<Test>::get(ALICE));
	});
}

#[test]
fn reporting_requires_the_manager_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tnpos::report_offence(RuntimeOrigin::signed(ALICE), BOB, Offence::Unavailable),
			pezsp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn an_offence_removes_the_member_from_the_seated_committee() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let victim = CurrentCommittee::<Test>::get()[0].clone();
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), victim.clone(),
			Offence::Equivocation));
		assert!(!CurrentCommittee::<Test>::get().contains(&victim));
	});
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezpallet-tnpos slash`
Expected: `report_offence` not found.

- [ ] **Step 3: Write the minimal implementation** — `src/slash.rs`:

```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! What an offence costs.
//!
//! Six of the nine strata are entered without capital, so a purely monetary penalty would
//! either be no deterrent there or would put a price on gates that are meant not to have
//! one. What is taken instead is standing: the member leaves the pool and cannot return for
//! a fixed number of eras. For someone whose place here is their record, that is the
//! heavier loss. The stake stratum keeps its existing staking-async slashing on top.

use crate::*;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// Eras out of the pool for failing to take part.
pub const BAN_UNAVAILABLE: u32 = 24;
/// Eras out of the pool for signing two conflicting blocks.
pub const BAN_EQUIVOCATION: u32 = 360;

/// What a member did.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Copy, PartialEq, Eq, Debug, TypeInfo,
	MaxEncodedLen,
)]
pub enum Offence {
	/// Seated but did not vote. A failure.
	#[codec(index = 0)]
	Unavailable,
	/// Signed two conflicting blocks. An attack: this is the act that can fork the chain.
	#[codec(index = 1)]
	Equivocation,
}

impl Offence {
	/// Eras banned from the pool.
	pub const fn ban_eras(&self) -> u32 {
		match self {
			Offence::Unavailable => BAN_UNAVAILABLE,
			Offence::Equivocation => BAN_EQUIVOCATION,
		}
	}
}

impl<T: Config> Pezpallet<T> {
	pub(crate) fn do_report_offence(who: T::AccountId, offence: Offence) -> DispatchResult {
		if let Some(stratum) = PoolMembers::<T>::take(&who) {
			StratumSize::<T>::mutate(stratum, |n| *n = n.saturating_sub(1));
		}

		// Leave the seated committee too. A member who equivocated must stop counting
		// towards quorum immediately, not at the end of the era.
		CurrentCommittee::<T>::mutate(|c| c.retain(|m| m != &who));

		let proposed = CurrentEra::<T>::get().saturating_add(offence.ban_eras());
		// A ban only ever lengthens. Recomputing it from whichever report arrived last would
		// let a trivial offence reported afterwards cut short the penalty for a serious one,
		// and equivocation costs fifteen times what going offline does.
		let banned_until =
			Banned::<T>::get(&who).map_or(proposed, |existing| existing.max(proposed));
		Banned::<T>::insert(&who, banned_until);
		Self::deposit_event(Event::Punished { who, offence, banned_until });
		Ok(())
	}
}
```

`src/lib.rs`: the storage item `Banned: StorageMap<_, Blake2_128Concat, T::AccountId, u32, OptionQuery>`;
`Error::Banned`; `Event::Punished { who, offence, banned_until }`; call index 5
`report_offence` (`ManagerOrigin`). **And a ban check at the head of `pool.rs::do_join`:**
```rust
		if let Some(until) = Banned::<T>::get(&who) {
			ensure!(CurrentEra::<T>::get() >= until, Error::<T>::Banned);
			Banned::<T>::remove(&who);
		}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezpallet-tnpos`
Expected: 26 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p pezpallet-tnpos
git add pezkuwi/pezpallets/tnpos/src
git commit -m "Punish offences by taking standing, scaled to what the member did"
```

---

## Task 12: committee handover (superseded 2026-08-29)

> **This task was completed as written and then undone, deliberately.** It produced
> `impl SessionManager for Tnpos`, which was right while the pallet lived on the relay. After
> the move to People it belonged to nobody: People's session is `CollatorSelection`'s and the
> chain the committee validates is two hops away. The implementation was removed and replaced
> by `SendCommitteeToRelay` — see "committee export" below. The steps are kept because the
> tests they describe still hold, rewritten to go through the export instead. A dead trait
> implementation is a claim the compiler keeps agreeing with.

## Task 12 (as originally written): `SessionManager` handover

**Files:**
- Modify: `pezkuwi/pezpallets/tnpos/src/lib.rs`, `src/tests.rs`

**Interfaces:**
- Produces: `impl pezpallet_session::SessionManager<T::AccountId> for Pezpallet<T>`

**Critical:** the deleted pallet's `SessionManager` implementation **was not wired into any
runtime** — `new_session` was never called even once, and shadow mode never ran. This task
writes the implementation; Task 14 actually **wires** it and verifies that it is wired.

- [ ] **Step 1: Write the failing test**

```rust
use pezpallet_session::SessionManager;

#[test]
fn a_new_session_hands_over_the_seated_committee() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let handed = <Tnpos as SessionManager<u64>>::new_session(1).expect("a committee exists");
		assert_eq!(handed, CurrentCommittee::<Test>::get().to_vec());
		assert_eq!(handed.len(), 27);
	});
}

#[test]
fn no_committee_hands_over_nothing_rather_than_an_empty_set() {
	// Returning Some(vec![]) would tell session to install an empty authority set and stop
	// the chain. None means "keep the current one", which is the recoverable answer.
	new_test_ext().execute_with(|| {
		assert!(<Tnpos as SessionManager<u64>>::new_session(1).is_none());
	});
}

#[test]
fn a_banned_member_is_never_handed_over() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let victim = CurrentCommittee::<Test>::get()[0].clone();
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), victim.clone(),
			Offence::Equivocation));
		let handed = <Tnpos as SessionManager<u64>>::new_session(2).unwrap();
		assert!(!handed.contains(&victim));
		assert_eq!(handed.len(), 26);
	});
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p pezpallet-tnpos session`
Expected: there is no `SessionManager` implementation.

- [ ] **Step 3: Write the minimal implementation** — into `src/lib.rs`:

```rust
	impl<T: Config> pezpallet_session::SessionManager<T::AccountId> for Pezpallet<T> {
		/// The seated committee, or `None` to keep the current authorities.
		///
		/// `None` rather than an empty vector: session reads `Some(vec![])` as an
		/// instruction to install no authorities, which stops the chain. When this pallet
		/// has nothing to offer, the safe answer is to change nothing.
		fn new_session(_index: u32) -> Option<Vec<T::AccountId>> {
			let c = CurrentCommittee::<T>::get();
			if c.is_empty() {
				log::warn!(target: "tnpos", "no committee seated; authorities unchanged");
				return None;
			}
			Some(c.to_vec())
		}

		fn end_session(_index: u32) {}

		fn start_session(_index: u32) {}
	}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p pezpallet-tnpos`
Expected: 29 tests PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt -p pezpallet-tnpos
git add pezkuwi/pezpallets/tnpos/src
git commit -m "Hand the seated committee to session, keeping authorities on failure"
```

---

## Task 13: Benchmarks and weights

**Files:**
- Create: `pezkuwi/pezpallets/tnpos/src/benchmarking.rs`
- Modify: `src/weights.rs`, `src/lib.rs`

**Interfaces:**
- Produces: a measured `WeightInfo` implementation

- [ ] **Step 1: Write the benchmarks** — `src/benchmarking.rs`:

```rust
// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarks. `seat_committee` is parameterised by pool size because it iterates
//! `PoolMembers` once per seated stratum; that iteration is the whole cost.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn join() {
		let who: T::AccountId = whitelisted_caller();
		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()), StratumId::Perwerde);
		assert!(PoolMembers::<T>::contains_key(&who));
	}

	#[benchmark]
	fn leave() {
		let who: T::AccountId = whitelisted_caller();
		Pezpallet::<T>::do_join(who.clone(), StratumId::Perwerde).unwrap();
		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()));
		assert!(!PoolMembers::<T>::contains_key(&who));
	}

	#[benchmark]
	fn seat_committee(p: Linear<45, { T::MaxPoolSize::get() }>) {
		// Fill every stratum evenly so all nine clear their floor and are actually drawn.
		for i in 0..p {
			let who: T::AccountId = account("member", i, 0);
			let stratum = StratumId::ALL[(i % 9) as usize];
			PoolMembers::<T>::insert(&who, stratum);
			StratumSize::<T>::mutate(stratum, |n| *n = n.saturating_add(1));
		}
		NextSeed::<T>::put([1u8; 32]);
		#[block]
		{
			let _ = Pezpallet::<T>::do_seat_committee();
		}
		assert!(!CurrentCommittee::<T>::get().is_empty());
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
```

- [ ] **Step 2: Verify the benchmark tests pass**

Run: `cargo test -p pezpallet-tnpos --features runtime-benchmarks`
Expected: the benchmark test suite PASSes.

- [ ] **Step 3: Measure the weights in CI**

**Do not run this on WSL.** Trigger the CI workflow for `pezpallet_tnpos` and bring in the
generated `weights.rs`. Expected: the `p` coefficient of `seat_committee` is greater than zero —
if it comes out constant, the iteration was not measured.

- [ ] **Step 4: Commit**

```bash
git add pezkuwi/pezpallets/tnpos/src/benchmarking.rs pezkuwi/pezpallets/tnpos/src/weights.rs
git commit -m "Measure TNPoS weights, with seating parameterised by pool size"
```

---

## Task 14: Wire it into Zagros and delete `validator-pool`

**Files:**
- Modify: `pezkuwi/runtime/zagros/src/lib.rs`, `pezkuwi/runtime/zagros/Cargo.toml`
- Delete: `pezkuwi/pezpallets/validator-pool/` (all of it),
  `pezkuwi/runtime/{zagros,pezkuwichain}/src/weights/pezpallet_validator_pool.rs`
- Modify: `Cargo.toml`, `pezkuwi/runtime/pezkuwichain/src/lib.rs` (the entry is removed)

**Critical:** this task does what the deleted pallet never did: it **actually wires**
`SessionManager`. Today `pezpallet_session::Config::SessionManager` is
`NoteHistoricalRoot<Self, StakingAhClient>`. On Zagros, TNPoS enters that chain.

- [ ] **Step 1: Write the test for the wiring** — into `pezkuwi/runtime/zagros/src/lib.rs`:

```rust
#[cfg(test)]
mod tnpos_wiring {
	use super::*;

	// The pallet this replaces implemented SessionManager and was never wired to anything,
	// so none of it ever ran. This asserts the type actually reaches session.
	#[test]
	fn tnpos_is_the_session_manager() {
		fn assert_is_manager<M: pezpallet_session::SessionManager<AccountId>>() {}
		assert_is_manager::<<Runtime as pezpallet_session::Config>::SessionManager>();
		let wired = core::any::type_name::<
			<Runtime as pezpallet_session::Config>::SessionManager,
		>()
		.to_lowercase();
		assert!(
			wired.contains("tnpos"),
			"TNPoS must be reachable from session, not merely compiled: {wired}"
		);
	}
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p zagros-runtime tnpos_wiring`
Expected: FAIL — `SessionManager` is still `StakingAhClient`.

- [ ] **Step 3: Wire the runtime**

In `pezkuwi/runtime/zagros/src/lib.rs`, **delete** the `StubTrustProvider`..`StubPerwerdeProvider`
block and the `pezpallet_validator_pool::Config` implementation, and in their place:

```rust
// =====================================================
// TNPOS CONFIGURATION
// =====================================================

/// Score source. Still a stub: the People-chain channel is M7.1 and the staking-score
/// oracle is M7.0, which blocks mainnet. Timestamps are current, so nothing reads as stale
/// while the real channel is absent.
pub struct StubScores;
impl pezkuwi_tnpos_primitives::scores::ScoreProvider<AccountId, BlockNumber> for StubScores {
	fn trust_of(_: &AccountId) -> ScoreSnapshot<BlockNumber> {
		ScoreSnapshot { value: 1_000, last_updated: System::block_number() }
	}
	fn tiki_of(_: &AccountId) -> ScoreSnapshot<BlockNumber> {
		ScoreSnapshot { value: 0, last_updated: System::block_number() }
	}
	fn perwerde_of(_: &AccountId) -> ScoreSnapshot<BlockNumber> {
		ScoreSnapshot { value: 0, last_updated: System::block_number() }
	}
	fn referral_of(_: &AccountId) -> ScoreSnapshot<BlockNumber> {
		ScoreSnapshot { value: 0, last_updated: System::block_number() }
	}
	fn staking_of(_: &AccountId) -> ScoreSnapshot<BlockNumber> {
		ScoreSnapshot { value: 0, last_updated: System::block_number() }
	}
}

parameter_types! {
	pub const TnposMaxScoreAge: BlockNumber = 4 * HOURS;
	pub const TnposEraLength: BlockNumber = 6 * HOURS;
	pub const TnposMaxPoolSize: u32 = 1_000;
}

impl pezpallet_tnpos::Config for Runtime {
	type WeightInfo = weights::pezpallet_tnpos::WeightInfo<Runtime>;
	type Sortition = pezpallet_tnpos::seed::CommitRevealSortition<Runtime>;
	type Scores = StubScores;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type MaxScoreAge = TnposMaxScoreAge;
	type EraLength = TnposEraLength;
	type MaxPoolSize = TnposMaxPoolSize;
}
```

`construct_runtime!`: replace `ValidatorPool: pezpallet_validator_pool = 91,` with
`Tnpos: pezpallet_tnpos = 91,`. Change it in the benchmark list too.

`pezpallet_session::Config`:
```rust
	type SessionManager = pezpallet_session::historical::NoteHistoricalRoot<Self, Tnpos>;
```

- [ ] **Step 4: Verify the test passes**

Run: `cargo test -p zagros-runtime tnpos_wiring`
Expected: PASS.

- [ ] **Step 5: Delete the old pallet and verify the build**

```bash
git rm -r pezkuwi/pezpallets/validator-pool
git rm pezkuwi/runtime/zagros/src/weights/pezpallet_validator_pool.rs
git rm pezkuwi/runtime/pezkuwichain/src/weights/pezpallet_validator_pool.rs
```
Remove the `pezkuwi/pezpallets/validator-pool` member and the `pezpallet-validator-pool`
dependency from `Cargo.toml`. Remove the pallet entry and the stub providers from the
`pezkuwichain` runtime (mainnet is **not wired to TNPoS yet** — M7.0 is open).

Run: `grep -rn "validator_pool\|validator-pool" --include="*.rs" --include="*.toml" .`
Expected: **no output.**

Run: `cargo check --workspace --all-targets`
Expected: SUCCESS.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
taplo format --config .config/taplo.toml Cargo.toml pezkuwi/runtime/zagros/Cargo.toml
git add -A
git commit -m "Wire TNPoS into Zagros session and remove pezpallet-validator-pool"
```

---

## Completion gate

Phase 1 is finished only when all of the following are verified:

- [ ] `cargo test -p pezkuwi-tnpos-primitives` — all passing
- [ ] `cargo build -p pezkuwi-tnpos-primitives --no-default-features` — `analysis` does not enter WASM
- [ ] `cargo test -p pezpallet-tnpos` — all passing
- [ ] `cargo test -p pezpallet-tnpos --features runtime-benchmarks`
- [ ] `cargo check --workspace --all-targets`
- [ ] `grep -rn "validator_pool\|validator-pool"` — no output
- [ ] `try-runtime` green against Zagros in CI
- [ ] The weights were measured on the CI runner, the `p` coefficient of `seat_committee` > 0
- [ ] The §5 table in `docs/TNPOS_DESIGN.md` matches the numbers produced by the test in Task 6
- [ ] M7.2 and M7.3 marked in `res/plans/PLAN.md` with their acceptance criteria
