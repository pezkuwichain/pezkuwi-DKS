# Release & runtime-upgrade runbook

PezkuwiChain holds real user funds. A runtime upgrade is the single most dangerous
operation we perform — a bad migration or a wrong WASM can corrupt state or brick the
chain. This runbook is the required, repeatable process. Do not skip steps.

## What a release produces

- **Runtime WASM blobs** for the deployable runtimes (relay `pezkuwichain`, system
  teyrchains `asset-hub-pezkuwichain`, `people-pezkuwichain`), built by
  `.github/workflows/release.yml`.
- For each blob: the **blake2-256 hash** (the value `authorizeUpgrade` commits to),
  the sha256, and an **SLSA build-provenance attestation**.
- A **draft GitHub release** (human review before publishing a fund-critical artifact).

The node binary is built from the same tag with the pinned toolchain.

## CI tiers (what gates what)

| Tier | When | Gates |
|---|---|---|
| Fast gate (`check`, `quick-checks`, `security`) | every push/PR | compile, 6-runtime WASM build, **try-runtime**, fmt, supply-chain (cargo-deny), zepter/taplo |
| Heavy suite (`test`) | nightly + `workflow_dispatch` | full nextest tests, workspace clippy, benches, doc-tests |
| Release (`release`) | tag `v*`/`runtime-*` + `workflow_dispatch` | builds WASM, hashes, attestation, draft release |

Before any mainnet upgrade the **heavy suite must be green on the release commit** —
trigger it with `workflow_dispatch` and wait for it; do not rely only on the fast gate.

## spec_version policy

- `spec_name` is the chain identity and **must never change** for a live chain.
- `spec_version` is a single monotonically increasing integer (schema `M_mmm_ppp`;
  current relay `1_020_007`). Bump it on **every** runtime change that ships — nodes use
  it to detect the upgrade.
- Bump `transaction_version` only when extrinsic encoding/call indices change.
- `impl_version` is informational.

## Runtime-upgrade procedure (mainnet, fund-critical)

1. **Freeze the change set** on `main`; fast gate green.
2. **Bump `spec_version`** (+ `transaction_version` if call encoding changed) in the
   affected runtime(s).
3. **Run the heavy suite** (`workflow_dispatch` → `test.yml`) on that commit → all green.
4. **try-runtime against live state** — dry-run the migrations on a snapshot of the live
   chain and confirm they succeed and post-upgrade invariants hold, *before* touching
   mainnet. (Pre-upgrade gate.)
5. **Tag** `runtime-<name>-vX` → `release.yml` builds the WASM and publishes the
   **blake2-256** hash + sha256 + attestation as a draft release.
6. **Reproduce & verify**: independently rebuild the tag with the pinned toolchain
   (`rust-toolchain.toml`) + committed `Cargo.lock`; confirm the blake2-256 matches.
   Publish the release only after the hash is confirmed.
7. **Authorize on-chain**: governance/sudo `system.authorizeUpgrade(<blake2-256>)`, then
   `system.applyAuthorizedUpgrade(<wasm>)`. The hash authorized on-chain must equal the
   one in the release.
8. **Watch**: confirm blocks are still produced and finalized after the upgrade.

## Shipped: 2026-07-10 mainnet upgrade (spec_version 1_020_010 / 1_020_011)

Deployed to all three teyrchains (relay, Asset Hub, People chain), each verified against
its own `System::LastRuntimeUpgrade` post-upgrade. Full account of what was found and
fixed along the way, including two release-pipeline bugs caught by independent
verification before anything reached mainnet: [docs/audits/2026-07-mainnet-upgrade.md](audits/2026-07-mainnet-upgrade.md).

The bundle:

- Token symbol standardized to **HEZ** (metadata-hash, claims `Prefix`, chain-spec).
- Claims statements repointed to **statement.pex.network** (hash-pinned; see
  [statement/README.md](statement/README.md)).
- `tiki` pallet: 12 new functional/professional roles.
- RuntimeVersion identity cleanup (dropped `parity-*` / `westmint`).
- **Security-audit fix set** (AI-assisted internal review; commits `14cbf1a`, `9afd99a`
  on `main`) — 11 findings closed across `identity-kyc`, `messaging`, `perwerde`,
  `presale`, `tiki`, `welati`, plus the relay `SendXcmOrigin` narrowing in
  `xcm_config.rs`. No storage migration required (all changes are new storage items or
  Config/logic-only).
  - `staking-score`: noter submissions now require a posted bond
    (`register_as_noter`/`unregister_as_noter`) and sit in a 1-hour dispute window
    (`DisputeWindow`) before taking effect, disputable by any Council member and
    slashable by `RootOrDiwanOrTechnical` on confirmed fraud. Root/XCM-Transact
    submissions are unaffected (chain-authenticated, not a personal key).
  - Fixed `MONTH_IN_BLOCKS`/`HOUR_IN_BLOCKS` in `staking-score`: an earlier revision
    had switched these to assume 12s blocks (5/min), citing an unused same-named
    constant from a different crate. `people-pezkuwichain` actually wires its
    `HOURS`/`DAYS`/`SlotDuration` from a 6s-block source — confirmed via live mainnet
    empirical block time, the real import path, and a try-runtime dry-run that
    panicked with an exact 2x slot mismatch under the wrong assumption. Every
    duration-tier threshold was filling twice as fast as intended; restored to 10
    blocks/min (6s), matching reality.
  - `staking-score`'s weights are now real benchmark output (previously manual
    estimates); `presale::withdraw_funds` had no benchmark case at all until
    commit `dc6d29e` — real weight pending a benchmark run.
- Dependency-security cleanup: `rand`/`tar`/`rpassword` patched (commit `2cbf2b3`);
  `wasmtime` (2 critical, aarch64-specific sandbox-escape CVEs; our fleet is x86_64)
  and a handful of other client-networking deps (`hickory-proto`, `rustls-webpki`,
  `yamux`) remain — each pinned by a deeper Substrate/Polkadot-SDK dependency and
  needing a coordinated major-version bump plus real node testing. Tracked as a
  separate task, not part of this bundle.

**Note on Zagros testnet validation:** the live "Zagros Testnet" chain (217.77.6.126)
runs the `pezkuwichain`/`asset-hub-pezkuwichain`/`people-pezkuwichain` runtime family
(spec_name confirmed via live RPC), not the `zagros`/`asset-hub-zagros`/`people-zagros`
family. The latter is a CI-checked-only reference implementation (never released — see
`release.yml`), left over from before this repo's clean-room rewrite. Bump
`spec_version` and build WASM from the `*-pezkuwichain` crates when validating a bundle
on Zagros before it ships to mainnet; the `*-zagros` crates are not deployed anywhere.

## Changing a claim statement

A statement edit changes its hash → changes the runtime → is a runtime upgrade and
invalidates prior signatures. Procedure: edit `docs/statement/*.html`, redeploy to
`statement.pex.network`, recompute sha256, update `StatementKind::to_text()`, bump
`spec_version`, ship. The repo copy, the served copy, and the runtime hash must stay
identical.

## Reproducibility note

Builds are pinned-toolchain reproducible (deterministic given `rust-toolchain.toml` +
`Cargo.lock` + flags). Fully hermetic srtool builds are pending a Rust 1.96 srtool image
(latest published is 1.93); the release pipeline will adopt srtool once available.
