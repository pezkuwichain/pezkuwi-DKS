# PezkuwiChain mainnet runtime upgrade — audit report

**Scope:** pezkuwi-DKS relay, Asset Hub, and People chain
**Window:** 2026-07-08 – 2026-07-10
**Findings:** 6 (0 unresolved)
**Fund impact:** none — zero loss, zero irreversible error
**Status:** all three chains upgraded and independently verified

## Executive summary

This is a full account of a routine mainnet runtime upgrade that became a
multi-stage engineering exercise: six real defects were found across the
runtime source, the release pipeline, and the verification tooling itself —
three pre-existing in the deployed runtime, two in the CI/release
infrastructure, and one in this audit's own first verification attempt. Every
defect was caught before it reached mainnet funds, because each step of
[RELEASE.md](../RELEASE.md)'s procedure was independently re-verified rather
than trusted on the first pass. No user funds were lost or put at risk at any
point.

## Final chain status

Verified against on-chain `System::LastRuntimeUpgrade`, not RPC display alone
(finding 6, below, is exactly why that distinction matters).

| Chain | Before | After | Notes |
|---|---|---|---|
| Relay | `1_020_007` | `1_020_010` | Clean on first attempt |
| Asset Hub | `1_020_008` | `1_020_010` | First attempt was a silent no-op (finding 6); corrected and reapplied |
| People chain | `1_020_009` | `1_020_011` | Clean on first attempt |

## Procedure followed

[`docs/RELEASE.md`](../RELEASE.md)'s 8-step fund-critical runbook:

1. **Freeze & bump** — change set frozen on `main`; `spec_version` bumped per runtime.
2. **Heavy suite** — full test suite, workspace clippy, and benches to green before any tag.
3. **Live-state dry run** — migrations rehearsed with `try-runtime` against a snapshot of real mainnet state.
4. **Tag & build** — a signed tag triggers a reproducible WASM build, hash, and SLSA build-provenance attestation.
5. **Independent reproduction** — the exact release artifact rebuilt from scratch, on separate hardware, before it is ever trusted.
6. **On-chain authorization** — `system.authorizeUpgrade(hash)` then `system.applyAuthorizedUpgrade(wasm)`, gated on explicit human sign-off per chain.
7. **Post-upgrade verification** — confirmed against the chain's own execution record, not just its RPC display.
8. **Watch** — confirmed blocks kept producing and finalizing on all three chains after each step.

## Findings

In the order each was discovered; severity reflects what would have happened
had it shipped unnoticed.

### 1. Staking-tier durations assumed the wrong block time — Medium

**What:** The `staking-score` pallet's tier-duration constants
(`MONTH_IN_BLOCKS`, `HOUR_IN_BLOCKS`) had been switched to assume 12-second
blocks (5/min) in an earlier revision, citing an unused same-named constant
from a different crate. `people-pezkuwichain` actually wires its
`HOURS`/`DAYS`/`SlotDuration` from a 6-second-block source.

**Impact:** Every staking duration tier filled twice as fast as intended — a
silent, compounding economic drift, not a crash risk.

**Confirmed via:** live mainnet empirical block time, the actual import path
in `people-pezkuwichain/src/lib.rs`, and a `try-runtime` live-state dry-run
that panicked with an exact 2x slot mismatch under the wrong assumption.

**Fix:** restored to 10 blocks/min (6s), matching reality (commit `16cbe65`).

### 2. Council pallet's storage version was never stamped, next to a migration that could have deleted it — High

**What:** `Council`'s on-chain storage version had been stuck at its genesis
value since deployment — the version-bump hook never ran. Sitting next to it
in the runtime's migration list was a `RemovePallet<CouncilPalletName>` entry,
almost certainly inherited unmodified from an upstream reference chain where
no council pallet was active under that name.

**Impact:** `Council` is a live, actively-configured pallet on this chain. Had
that removal migration ever executed, it would have deleted a governance
pallet currently in use.

**Fix:** replaced the removal with a no-op `VersionedMigration<0,4,...>` that
only stamps the correct storage version; deleted the removal entry entirely
(commit `0fd85f6`).

### 3. 12.49M HEZ sat in undecodable "ghost" holds from a removed pallet — High

**What:** Twenty-five accounts carried a `Balances::Holds` entry tagged with a
hold-reason discriminant that no longer corresponds to any pallet in the
current runtime — left behind when an old staking pallet was removed without
releasing its holds. Any code path that tried to decode these accounts' holds
would panic.

**Impact:** a latent crash risk on any future storage-decoding operation
touching these accounts, and 12.49M HEZ nominally "locked" against nothing.

**Investigation:** pulled the live `RuntimeHoldReason` enum directly from
on-chain metadata and hand-decoded the raw SCALE bytes for all 25 accounts.
Most had little or no real reserved balance behind the stale record — the
funds had already found their way loose through normal channels over time,
leaving only a bookkeeping ghost.

**Fix:** a migration that releases exactly `min(claimed, actually-reserved)`
per account and always clears the stale record — never invents balance that
isn't really there (commit `8c9fb93`). Dry-run against a full live-state
snapshot: 0 errors, all 63 pallets' try-state checks passed.

### 4. Release pipeline could publish a test-only WASM variant as the production relay runtime — High

**What:** the relay runtime's `build.rs` always compiles two WASM blobs side
by side: the real one, and a `fast-runtime`-feature devnet variant with
shortened governance and staking periods, meant only for local test networks.
The release workflow's artifact-selection step (`find ... | head -1`) picked
whichever file its directory listing happened to return first — not a
reliable distinction between the two.

**Impact:** on the first release build of this upgrade, the picker selected
the devnet variant. Its hash was computed, signed, and staged as the official
relay artifact. Had it gone unnoticed past the independent-reproduction step,
authorizing it on-chain would have silently swapped mainnet's governance and
staking timers for local-test values.

**Caught by:** step 5 of the procedure (independent, from-scratch rebuild),
which surfaced a hash mismatch against the published artifact once the
correct file was identified by hand.

**Fix:** the picker now explicitly excludes the devnet variant and fails
loudly, rather than guessing, if more than one candidate remains (PR #27).
Never published, never authorized — caught entirely inside the verification
stage.

### 5. Release builds were not yet reproducible across the CI fleet — Medium

**What:** these builds are pinned-toolchain reproducible, not yet fully
hermetic (a known, already-documented limitation pending upstream `srtool`
support for the pinned Rust version). The self-hosted build fleet doesn't
share one filesystem layout, and the absolute checkout path measurably
changes the compiled WASM's bytes. Two idle runners building the identical
commit produced different hashes.

**Impact:** not a correctness bug in the shipped code, but a real
trust-and-verifiability gap: an outside party rebuilding at a different path
than the one that happened to build the release would see a mismatched hash
and could reasonably suspect tampering that wasn't there.

**Fix:** release builds are now pinned to one specific, named machine, making
the published hash reliably reproducible until hermetic builds are available
(PR #28).

### 6. Asset Hub's version number was never bumped — its first upgrade attempt did nothing — High

**What:** the runtime bundle for this release rebuilt and redeployed Asset
Hub's WASM, but its source-level `spec_version` was left unchanged. The
on-chain code was successfully replaced — but because the chain's own
version bookkeeping saw no version change, it correctly ran no migrations
and left its execution behavior untouched.

**Impact:** none to funds or safety — the swap was between two builds the
chain considered identical. But the upgrade was, in practice, a no-op.

**Caught by:** post-upgrade verification checking the chain's own execution
record (`System::LastRuntimeUpgrade`) rather than trusting that a successful
transaction implies a successful upgrade.

**Fix:** version bumped in source (PR #29), rebuilt, independently
re-verified, and reapplied as a second, corrected on-chain transaction.
Confirmed genuine on the second attempt via the same strict check.

## Independent verification, chain by chain

Every published hash was rebuilt from the tagged source and matched, byte
for byte, before being trusted.

| Runtime | Published blake2-256 | Rebuilt independently | On-chain result |
|---|---|---|---|
| Relay (`pezkuwichain-runtime`) | `0x00399247…7494` | Match | Confirmed live — `1_020_010` |
| Asset Hub (corrected build) | `0xf2084e9f…7b5e` | Match | Confirmed live — `1_020_010` |
| People chain | `0x0b7d4e34…5a5` | Match | Confirmed live — `1_020_011` |

## Methodology note

A recurring pattern across this audit: the convenient check and the correct
check gave different answers more than once. RPC endpoints and quick version
queries reflect what a node *believes*, which can lag or mislead; the
chain's own storage — the executed record, not its self-report — was
treated as the only ground truth whenever the two were in tension. This
discipline is what surfaced findings 4 and 6, both of which a faster, more
trusting process would have missed.

The mainnet Sudo key used to authorize each upgrade is held offline and was
verified against the chain's own `Sudo::Key` storage before any transaction
was signed with it — not assumed correct from a previously recorded
reference, which in this case had itself gone stale.

A secondary, host-specific pitfall is worth recording for future
verification passes: on at least one build host, the `root` account's
default Rust toolchain differed from the pinned one the CI service account
actually uses, producing a wrong-toolchain (not wrong-path) hash mismatch
during a rebuild attempt. Verification builds on any host should confirm
`rustc --version` matches `rust-toolchain.toml` for the account actually
running the build, not assume it from another account on the same machine.
