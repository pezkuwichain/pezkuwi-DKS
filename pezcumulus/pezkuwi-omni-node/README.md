# PezkuwiChain Omni Node

This is a white labeled implementation based on `pezkuwi-omni-node-lib`.
It can be used to start a teyrchain node from a provided chain spec file. It is only compatible with runtimes that use block
number `u32` and `Aura` consensus.

## Installation

Download & expose it via `PATH`:

```bash
# Download and set it on PATH.
wget https://github.com/pezkuwichain/pezkuwi-sdk/releases/download/<stable_release_tag>/pezkuwi-omni-node
chmod +x pezkuwi-omni-node
export PATH="$PATH:`pwd`"
```

> Replace `<stable_release_tag>` with the latest stable tag from the [Pezkuwi SDK releases](https://github.com/pezkuwichain/pezkuwi-sdk/releases)
>
> For example:
> ```bash
> wget https://github.com/pezkuwichain/pezkuwi-sdk/releases/download/pezkuwi-stable2506-1/pezkuwi-omni-node
> ```

Compile & install via `cargo`:

```bash
# Assuming ~/.cargo/bin is on the PATH
cargo install pezkuwi-omni-node --locked
```

## Usage

A basic example for an Omni Node run starts from a runtime which implements the [`sp_genesis_builder::GenesisBuilder`](https://docs.rs/pezsp-genesis-builder/latest/sp_genesis_builder/trait.GenesisBuilder.html).
The interface mandates the runtime to expose a [`named-preset`](https://docs.rs/pezstaging-chain-spec-builder/latest/staging_chain_spec_builder/#generate-chain-spec-using-runtime-provided-genesis-config-preset).

### 1. Install chain-spec-builder

**Note**: `chain-spec-builder` binary is published on [`crates.io`](https://crates.io) under
[`pezstaging-chain-spec-builder`](https://crates.io/crates/pezstaging-chain-spec-builder) due to a name conflict.
Install it with `cargo` like bellow :

```bash
cargo install pezstaging-chain-spec-builder --locked
```

### 2. Generate a chain spec

Omni Node requires the chain spec to include a JSON key named `relay_chain`. It is set to a chain id,
representing the chain name, e.g. `zagros`, `paseo`, `pezkuwichain`, `pezkuwi`, or `dicle`, but
there are also local variants that can be used for testing, like `pezkuwichain-local` or `zagros-local`. The
local variants are available only for a build of `pezkuwi-omni-node` with
`zagros-native` and `pezkuwichain-native` features respectively.

<!-- TODO: https://github.com/pezkuwichain/pezkuwi-sdk/issues/156 -->
Additionaly, the `--para-id` flag can be used to set the JSON key named `para_id`. This flag is used
by nodes to determine the teyrchain id, and it is especially useful when the teyrchain id can not be
fetched from the runtime, when the state points to a runtime that does not implement the
`cumulus_primitives_core::GetTeyrchainInfo` runtime API. It is recommended for runtimes to implement
the runtime API and be upgraded on chain.

Example command bellow:

```bash
chain-spec-builder create --relay-chain <relay_chain_id> --para-id <id> -r <runtime.wasm> named-preset <preset_name>
```

### 3. Run Omni Node

And now with the generated chain spec we can start the node in development mode like so:

```bash
pezkuwi-omni-node --dev --chain <chain_spec.json>
```

## Useful links

* [`Omni Node Pezkuwi SDK Docs`](https://docs.pezkuwichain.io/sdk/master/pezkuwi_sdk_docs/reference_docs/omni_node/index.html)
* [`Chain Spec Genesis Reference Docs`](https://docs.pezkuwichain.io/sdk/master/pezkuwi_sdk_docs/reference_docs/chain_spec_genesis/index.html)
* `pezkuwi-teyrchain-bin`
* [`pezkuwi-sdk-teyrchain-template`](https://github.com/pezkuwichain/pezkuwi-sdk-teyrchain-template)
* [`frame-omni-bencher`](https://crates.io/crates/frame-omni-bencher)
* [`pezstaging-chain-spec-builder`](https://crates.io/crates/pezstaging-chain-spec-builder)
