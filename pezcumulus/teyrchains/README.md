# Teyrchains

This directory is the home of Parity-developed teyrchain runtimes. This directory is _runtime
focused_, and does not include builds of teyrchain _nodes_.

The general internal structure is:

- `chain-specs`: Chain specs for the runtimes contained in its sibling dir `runtimes`.
- `common`: Common configurations, `impl`s, etc. used by several teyrchain runtimes.
- `integration-tests`: Integration tests to test teyrchain interactions via XCM.
- `pallets`: FRAME pallets that are specific to teyrchains.
- `runtimes`: The entry point for teyrchain runtimes.

## System Teyrchains

The `runtimes` directory includes many, but is not limited to,
[system teyrchains](https://wiki.network.pezkuwichain.io/docs/learn-system-chains). Likewise, not all
system teyrchains are in this repo.

## Releases

The project maintainers generally try to release a set of teyrchain runtimes for each PezkuwiChain
Relay Chain runtime release.
