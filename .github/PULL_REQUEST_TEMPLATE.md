# Description

<!-- What does this PR change, and why? Link any related issue. -->

## Type of change

- [ ] Runtime logic (relay / system parachain)
- [ ] Original pallet (`pezpallet-*`)
- [ ] Framework / dependency
- [ ] Docs / tooling / CI

## Runtime impact

- [ ] This changes on-chain logic / storage / weights
- [ ] `spec_version` bumped (if a runtime upgrade)
- [ ] Storage migration included and tested (if needed)
- [ ] No runtime impact

## Checklist

- [ ] `cargo fmt --all` is clean
- [ ] The affected runtime(s) build (`cargo check -p <runtime>`)
- [ ] Tests added / updated where relevant
- [ ] For fund-affecting changes: reviewed for overflow, weight, and access-control correctness

> ⚠️ This repository powers a live, fund-holding network. Changes to balances, XCM,
> bridges, or weights require extra scrutiny.
