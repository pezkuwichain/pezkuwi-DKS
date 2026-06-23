# PezkuwiChain — Token & Network Standard

This is the single source of truth for token symbols across PezkuwiChain networks.
Any new chain-spec, runtime, or document must follow it.

## Principle

A system teyrchain (Asset Hub, People, Coretime, Bridge Hub) does **not** have its own
native token — it shares the relay chain's native token for fees and existential
deposits. Its `tokenSymbol` therefore equals the relay's symbol (exactly as
`asset-hub-polkadot` reports `DOT`, not a symbol of its own).

## Mainnet — PezkuwiChain

| Item | Value |
|------|-------|
| Relay native token | **HEZ** |
| Decimals | 12 (`UNITS = 10^12`) |
| System teyrchains (asset-hub / people / coretime / bridge-hub-pezkuwichain) | **HEZ** (shared relay native) |
| Asset Hub governance token | **PEZ** (a token/asset on Asset Hub — 5,000,000,000 supply; *not* the native fee token) |
| ss58Format | 42 (generic; registering a dedicated prefix is tracked separately) |

`HEZ` total issuance on the relay: 200,000,000. Genesis allocation constants are
named `HEZ_*`. Where an amount is just a base-unit multiple in a non-native context
(e.g. PEZ allocations on Asset Hub), use plain `UNITS`, not a token-named alias.

## Testnets & dev networks

| Network | Symbol | Notes |
|---------|--------|-------|
| Zagros | **ZGR** | Primary public testnet |
| Dicle | **DCL** | Testnet |
| Versi | **VRS** | Staging/dev network |
| Paseo | **PAS** | Dev network |

Testnet symbols are intentionally distinct from `HEZ` so that testnet tokens can never
be confused with mainnet value — this is a deliberate safety property, not an
inconsistency. Testnet system teyrchains share their own relay's symbol (e.g. the
Zagros system teyrchains report `ZGR`).

## Notes for maintainers

- The token symbol is embedded in three runtime-level places, so changing it on a live
  network requires a coordinated runtime upgrade + chain-spec redeploy:
  1. `build.rs` → `enable_metadata_hash("HEZ", 12)` (drives the CheckMetadataHash
     extension used by hardware/offline signers).
  2. The Claims `Prefix` signed-payload (`b"Pay HEZ to the Pezkuwichain account:"`).
  3. The chain-spec `properties.tokenSymbol`.
- `TYR` was an earlier placeholder that had leaked into the mainnet runtimes/specs; it
  has been fully standardized to `HEZ`. Do not reintroduce it.
