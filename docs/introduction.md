# Introduction to Pezkuwi Network

Welcome to Pezkuwi Network - a next-generation blockchain ecosystem built on Bizinikiwi, designed to empower communities with decentralized governance, identity management, and economic sovereignty.

**Last Updated:** 2025-12-10
**Version:** 3.0.0
**Status:** PHASE 1 COMPLETE | BENCHMARKS COMPLETE | PHASE 2 IN PROGRESS

---

## Table of Contents

1. [Overview](#1-overview)
2. [Tokenomics Architecture](#2-tokenomics-architecture)
3. [Network Topology](#3-network-topology)
4. [Technical Implementation](#4-technical-implementation)
5. [Development Roadmap](#5-development-roadmap)
6. [Security Framework](#6-security-framework)

---

## 1. Overview

### 1.1. Mission

Pezkuwi Network aims to provide a secure, consistent, and scalable blockchain infrastructure that enables:

- **Decentralized Governance** through the Welati pezpallet
- **Identity & Citizenship** via zero-knowledge KYC verification
- **Community-Driven Economics** with dual-token system
- **Education Platform** through Perwerde pezpallet
- **Trust-Based Networking** with reputation scoring

### 1.2. Core Features

| Feature | Description | Status |
| --- | --- | --- |
| **Dual Token System** | HEZ (native gas) and PEZ (governance) tokens | ✅ Complete |
| **Asset Hub** | PEZ (ID:1) and wHEZ (ID:2) tokens at genesis | ✅ Complete |
| **Identity Bootstrap** | Founder citizenship with IdentityKyc initialization | ✅ Complete |
| **Validator Infrastructure** | TNPoS consensus with configurable validator sets | ✅ Complete |
| **Treasury System** | PezTreasury pezpallet with 5B PEZ initial supply | ✅ Complete |
| **Synthetic Halving** | 48-month halving mechanism for reward distribution | ✅ Complete |

### 1.3. Network Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    PEZKUWICHAIN RELAY CHAIN                  │
│                     (100 Validators - Mainnet)               │
│                                                              │
│  Native Token: HEZ (Inflationary)                           │
│  Consensus: TNPoS (Trust-enhanced NPoS)                     │
│  Block Time: ~6 seconds                                     │
└─────────────────────┬───────────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┐
        ▼             ▼             ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│  ASSET HUB    │ │ PEOPLE CHAIN  │ │ BRIDGE HUB    │
│  (ParaId:1000)│ │ (ParaId:1004) │ │ (ParaId:1002) │
├───────────────┤ ├───────────────┤ ├───────────────┤
│ PEZ (ID:1)    │ │ IdentityKyc   │ │ XCM Bridge    │
│ wHEZ (ID:2)   │ │ Welati (Gov)  │ │ Zagros Link   │
│ Presale       │ │ Perwerde      │ │               │
│ TokenWrapper  │ │ Trust         │ │               │
│ PezTreasury   │ │               │ │               │
└───────────────┘ └───────────────┘ └───────────────┘
```

---

## 2. Tokenomics Architecture

### 2.1. Dual Token System

#### HEZ Token (Native - Relay Chain)

| Property | Value |
| --- | --- |
| Type | Native Balance (Inflationary) |
| **Genesis Supply** | **200,000,000 HEZ** (200M) |
| Inflation | Dynamic NPoS per era (~10%/year target) |
| Usage | Gas, Staking, Transaction Fees |
| Decimals | 12 |
| Unit | 1 HEZ = 10^12 Planck |

### 2.2. HEZ Genesis Distribution

```
┌────────────────────────────────────────────────────────────────┐
│                    HEZ GENESIS SUPPLY: 200,000,000             │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  Founder       10%   │
│  ██████████████████████████████████████░░  Presale      50%   │
│  ████████████████░░░░░░░░░░░░░░░░░░░░░░░░  Gov Treasury  20%   │
│  ████████████████░░░░░░░░░░░░░░░░░░░░░░░░  Airdrop       20%   │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

| Category | Percentage | Amount (HEZ) | Description |
| --- | --- | --- | --- |
| Founder | 10% | 20,000,000 | Project founder allocation |
| Presale | 50% | 100,000,000 | Early investors |
| Government Treasury | 20% | 40,000,000 | Governance fund |
| Airdrop | 20% | 40,000,000 | Community distribution |
| **TOTAL** | **100%** | **200,000,000** | |

> **Note:** Post-genesis, HEZ supply increases ~10% annually through NPoS inflation.

#### PEZ Token (Asset - Asset Hub)

| Property | Value |
| --- | --- |
| Asset ID | 1 |
| Type | pallet_assets (Fixed Supply) |
| Total Supply | **5,000,000,000 PEZ** (5 Billion) |
| Decimals | 12 |
| Unit | 1 PEZ = 10^12 Planck |
| Halving | Synthetic - 50% reduction every 48 months |

### 2.3. PEZ Token Distribution

```
┌────────────────────────────────────────────────────────────────┐
│                    PEZ TOTAL SUPPLY: 5,000,000,000             │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  ████████████████████░░░░░░░░░░░░░░░░░░░░  Treasury    20.25%  │
│  ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  Presale      1.875% │
│  ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  Founder      1.875% │
│  ████████████████████████████████████████  Rewards     76.00%  │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

| Category | Percentage | Amount (PEZ) | Vesting/Lock |
| --- | --- | --- | --- |
| Treasury (Governance) | 20.25% | 1,012,500,000 | Governance controlled |
| Presale | 1.875% | 93,750,000 | Presale pezpallet managed |
| Founder | 1.875% | 93,750,000 | 4-year vesting |
| Validator/Nominator Rewards | 76.00% | 3,800,000,000 | Synthetic halving distribution |

### 2.4. Synthetic Halving Mechanism

```rust
// pezpallet-pez-treasury: lib.rs
const HALVING_PERIOD_MONTHS: u32 = 48;
const INITIAL_EPOCH_REWARD: u128 = 79_166_666 * PEZ; // ~79M PEZ per month

fn calculate_epoch_reward(current_month: u32) -> u128 {
    let halving_count = current_month / HALVING_PERIOD_MONTHS;
    INITIAL_EPOCH_REWARD >> halving_count // Halves every 48 months
}
```

**Distribution Schedule:**

| Year | Monthly Distribution | Annual Total | Cumulative |
| --- | --- | --- | --- |
| 1-4 | ~79.17M PEZ | ~950M PEZ | ~3.8B PEZ |
| 5-8 | ~39.58M PEZ | ~475M PEZ | ~4.75B PEZ |
| 9-12 | ~19.79M PEZ | ~237.5M PEZ | ~4.99B PEZ |
| 13+ | Decreasing | ... | ~5B PEZ |

---

## 3. Network Topology

### 3.1. Validator Roadmap

| Phase | Validator Count | Collators (Parachain) | Description |
| --- | --- | --- | --- |
| Dev | 1 | 1 | Single node development |
| Local | 2 | 2 | Alice + Bob |
| Alpha | 4 | 2 | Core team testing |
| Beta | 8 | 4 | Real keys testing |
| Staging | 21 | 8 | Performance testing |
| Mainnet | 100 | 16 | Live network launch |

### 3.2. Parachain IDs

| Parachain | ID | Status |
| --- | --- | --- |
| Asset Hub Pezkuwichain | 1000 | ✅ Defined |
| Bridge Hub Pezkuwichain | 1002 | ✅ Complete |
| People Pezkuwichain | 1004 | ✅ Defined |
| Coretime/Broker | 1005 | ✅ Defined |

---

## 4. Technical Implementation

### 4.1. Custom Pallets

Pezkuwi Network includes 12 custom pallets:

| # | Pezpallet | Purpose | Benchmarks |
| --- | --- | --- | --- |
| 1 | pezpallet-presale | Token launch platform | ✅ Complete |
| 2 | pezpallet-identity-kyc | KYC verification (6 extrinsics) | ✅ Complete |
| 3 | pezpallet-welati | Democratic governance | ✅ Complete |
| 4 | pezpallet-perwerde | Education platform (4 extrinsics) | ✅ Complete |
| 5 | pezpallet-pez-treasury | Community treasury | ✅ Complete |
| 6 | pezpallet-pez-rewards | Staking rewards (6 extrinsics) | ✅ Complete |
| 7 | pezpallet-validator-pool | Validator management | ✅ Complete |
| 8 | pezpallet-staking-score | Reputation metrics (1 extrinsic) | ✅ Complete |
| 9 | pezpallet-trust | P2P trust system (3 extrinsics) | ✅ Complete |
| 10 | pezpallet-referral | Referral incentives | ✅ Complete |
| 11 | pezpallet-tiki | NFT citizenship (4-tier) | ✅ Complete |
| 12 | pezpallet-token-wrapper | Cross-chain wrapping | ✅ Complete |

### 4.2. Asset Hub Genesis Configuration

```rust
// AssetsConfig for PEZ and wHEZ
assets: AssetsConfig {
    assets: vec![
        (PEZ_ASSET_ID, treasury_account.clone(), true, 1),
        (WHEZ_ASSET_ID, treasury_account.clone(), true, 1),
    ],
    metadata: vec![
        (PEZ_ASSET_ID, b"Pez Token".to_vec(), b"PEZ".to_vec(), 12),
        (WHEZ_ASSET_ID, b"Wrapped HEZ".to_vec(), b"wHEZ".to_vec(), 12),
    ],
    accounts: vec![
        (PEZ_ASSET_ID, treasury_account.clone(), TREASURY_ALLOCATION + REWARDS_POOL),
        (PEZ_ASSET_ID, founder_account.clone(), FOUNDER_ALLOCATION),
        (PEZ_ASSET_ID, presale_account.clone(), PRESALE_ALLOCATION),
    ],
    next_asset_id: Some(3),
},
```

### 4.3. People Chain Genesis Configuration

```rust
// IdentityKyc - Founder citizen
identity_kyc: IdentityKycConfig {
    initial_citizens: vec![
        (
            founder_account.clone(),
            pallet_identity_kyc::CitizenInfo {
                kyc_level: pallet_identity_kyc::KycLevel::Full,
                registration_block: 0,
                referrer: None,
                is_founder: true,
            },
        ),
    ],
},
```

---

## 5. Development Roadmap

### 5.1. Phase 1: Core Infrastructure ✅ COMPLETE

| Task | Status |
| --- | --- |
| Asset Hub AssetsConfig implementation | ✅ Complete |
| People Chain IdentityKycConfig implementation | ✅ Complete |
| Relay Chain HEZ genesis distribution | ✅ Complete |
| Bridge Hub Parachain ID fix (1013→1002) | ✅ Complete |
| Dev preset compile tests | ✅ Complete |
| All custom pezpallet benchmarks | ✅ Complete |
| Weight generation (real values) | ✅ Complete |

### 5.2. Phase 2: Testnet Presets (In Progress)

| Task | Status |
| --- | --- |
| alpha_testnet preset | ⬜ Pending |
| beta_testnet preset (8 validators) | ⬜ Pending |
| Validator key integration | ⬜ Pending |

### 5.3. Phase 3: Staging & Mainnet

| Task | Status |
| --- | --- |
| staging_testnet preset (21 validators) | ⬜ Pending |
| mainnet preset (100 validators) | ⬜ Pending |
| Zombienet test scenarios | ⬜ Pending |

### 5.4. Phase 4: Validation & Documentation

| Task | Status |
| --- | --- |
| Build-spec tests for all presets | ⬜ Pending |
| E2E tests | ⬜ Pending |
| Operator documentation | ⬜ Pending |

---

## 6. Security Framework

### 6.1. Key Management Principles

1. **NEVER** write seed phrases in source code
2. **ONLY** use public key hex values in genesis
3. **NEVER** commit key files to git (.gitignore)
4. Store seed backups **OFFLINE** and **ENCRYPTED**
5. Protect mainnet keys with **HSM** or **Vault**

### 6.2. Pre-Mainnet Checklist

- [x] All keys verified and unique
- [x] No seed phrases in code
- [x] Key files in .gitignore
- [x] Treasury account under governance control
- [x] Copyright update complete
- [ ] Security audit complete
- [ ] Bug bounty program active
- [ ] Sudo removal plan ready

### 6.3. Contact & Support

- **Website:** https://pezkuwichain.io
- **Documentation:** https://docs.pezkuwichain.io
- **GitHub:** https://github.com/pezkuwichain
- **Technical Support:** Discord #validators-tech

---

## Getting Started

Ready to explore Pezkuwi Network? Here are your next steps:

1. **[SDK Documentation](/docs/sdk)** - Explore our Rust SDK
2. **[Whitepaper](/docs/whitepaper)** - Deep dive into our vision
3. **[GitHub](https://github.com/pezkuwichain/pezkuwi-sdk)** - View source code
4. **[Explorer](https://explorer.pezkuwichain.io)** - Browse the blockchain

---

*Welcome to the future of decentralized governance.*

---
*Last updated: 2025-12-10 | Version: 3.0.0*
