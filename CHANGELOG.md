# Changelog

All notable changes to Pezkuwi SDK are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

Changes merged to `main` since 0.44.0, by pull request. Runtime upgrades are identified by `spec_version`.

### Added
- Zagros rebuilt as a validated mirror of mainnet, with its own genesis accounts, custody and bootnodes (#46, #71, #108)
- The founding sequence of the state (citizenship, parliament, court, offices) rehearsed on zombienet and proved against
  live chains (#87, #88, #90, #92)
- Asset Hub era clock; the committee is paid by trust × work (#104)
- Airdrop pot behind a three-signature authority; each fund placed under the authority that spends it (#70, #80)
- Election rounds that stall past a timeout are abandoned (#40, #42)
- Measured weights for every custom pallet and for Asset Hub treasury and OpenGov, taken on the reference host
  (#89, #91, #96, #102, #105, #110, #111); welati calls get weights of their own (#112)
- OSV advisory gate over every resolved crate and every committed npm lockfile (#72, #77, #113)
- `bizinikiwi --dev` node image published to GHCR (#109)

### Changed
- Framework synchronized to polkadot-stable2606-1 (#65, #74)
- Referral claims are unlimited and fee-paid; settlement costs the same however many claims exist, and leftover claims
  are cleared by `on_idle` (#112)
- The Fellowship is retired (#97); `XcmPassthrough` is removed from the Zagros relay (#111)
- Release builds are hermetic and pinned to one runner for reproducible hashes (#28, #97)
- CI actions are pinned to commit SHAs and container base images to digests (#112)
- Opt-in GitLab CI pipeline, unified build image, nextest partitioning, rustdoc and mdbook (#36)

### Fixed
- Approved bounties were never funded: the Asset Hub treasury had no `SpendFunds` hook (#110)
- Staking elections broke on `U128CurrencyToVote` (#31); the paged voter cursor was dropped mid-election (#39)
- Zagros announced itself as mainnet (#61)
- The release job could pick the relay's fast-runtime devnet WASM as the release artifact (#27)

### Security
- Every RustSec advisory in the dependency graph closed before genesis (#107); rustls 0.23.46 and wasmtime 36.0.18 for
  RUSTSEC-2026-0346..0349 and RUSTSEC-2026-0363 (#112)
- Job-level workflow token permissions, a hashed Python lock for CI tooling, and removal of committed `.pyc` files (#112)

### Removed
- The contracts pallet family (#110) and the dead local staking stack on the Zagros relay (#43)

## 0.44.0 — 2026-04-21

### Added
- Pezkuwi mainnet launch
- Asset Hub with nomination pools staking
- People Chain with IdentityKyc, Tiki, Trust, Referral, Nfts pallets
- Zagros testnet environment
- TNPoS (Trust-Nominated Proof of Stake) consensus
- HEZ token (12 decimals)

### Changed
- Fork of Polkadot SDK — all Polkadot/Substrate references rebranded to Pezkuwi/Bizinikiwi/Pezcumulus
- Signing context changed to "bizinikiwi"
- SS58 prefix updated for Pezkuwi network

### Attribution
This SDK is derived from [polkadot-sdk](https://github.com/paritytech/polkadot-sdk)
by Parity Technologies (UK) Ltd, licensed under Apache-2.0 and GPL-3.0-only.
See [NOTICE](./NOTICE) for full attribution.

---

[Unreleased]: https://github.com/pezkuwichain/pezkuwi-DKS/commits/main

