# Bizinikiwi Node Template

A fresh [Bizinikiwi](https://bizinikiwi.pezkuwichain.io/) node, ready for hacking :rocket:

A standalone version of this template is available for each release of PezkuwiChain
in the [Bizinikiwi Developer Hub Teyrchain
Template](https://github.com/bizinikiwi-developer-hub/bizinikiwi-node-template/)
repository. The teyrchain template is generated directly at each PezkuwiChain
release branch from the [Solochain Template in
Bizinikiwi](https://github.com/pezkuwichain/pezkuwi-sdk/tree/main/templates/solochain)
upstream

It is usually best to use the stand-alone version to start a new project. All
bugs, suggestions, and feature requests should be made upstream in the
[Bizinikiwi](https://github.com/pezkuwichain/pezkuwi-sdk/tree/main/bizinikiwi)
repository.

## Getting Started

Depending on your operating system and Rust version, there might be additional
packages required to compile this template. Check the
[Install](https://docs.pezkuwichain.io/install/) instructions for your platform for
the most common dependencies. Alternatively, you can use one of the [alternative
installation](#alternatives-installations) options.

Fetch solochain template code:

```sh
git clone https://github.com/pezkuwichain/pezkuwi-sdk/issues/25.git solochain-template

cd solochain-template
```

### Build

🔨 Use the following command to build the node without launching it:

```sh
cargo build --release
```

### Embedded Docs

After you build the project, you can use the following command to explore its
parameters and subcommands:

```sh
./target/release/pez-solochain-template-node -h
```

You can generate and view the [Rust
Docs](https://doc.rust-lang.org/cargo/commands/cargo-doc.html) for this template
with this command:

```sh
cargo +nightly doc --open
```

### Single-Node Development Chain

The following command starts a single-node development chain that doesn't
persist state:

```sh
./target/release/pez-solochain-template-node --dev
```

To purge the development chain's state, run the following command:

```sh
./target/release/pez-solochain-template-node purge-chain --dev
```

To start the development chain with detailed logging, run the following command:

```sh
RUST_BACKTRACE=1 ./target/release/pez-solochain-template-node -ldebug --dev
```

Development chains:

- Maintain state in a `tmp` folder while the node is running.
- Use the **Alice** and **Bob** accounts as default validator authorities.
- Use the **Alice** account as the default `sudo` account.
- Are preconfigured with a genesis state (`/node/src/chain_spec.rs`) that
  includes several pre-funded development accounts.


To persist chain state between runs, specify a base path by running a command
similar to the following:

```sh
// Create a folder to use as the db base path
$ mkdir my-chain-state

// Use of that folder to store the chain state
$ ./target/release/pez-solochain-template-node --dev --base-path ./my-chain-state/

// Check the folder structure created inside the base path after running the chain
$ ls ./my-chain-state
chains
$ ls ./my-chain-state/chains/
dev
$ ls ./my-chain-state/chains/dev
db keystore network
```

### Connect with PezkuwiChain-JS Apps Front-End

After you start the node template locally, you can interact with it using the
hosted version of the [PezkuwiChain/Bizinikiwi
Portal](https://pezkuwichain.io/#/explorer?rpc=ws://localhost:9944)
front-end by connecting to the local node endpoint. A hosted version is also
available on [IPFS](https://dotapps.io/). You can
also find the source code and instructions for hosting your own instance in the
[`pezkuwi-js/apps`](https://github.com/polkadot-js/apps) repository.

### Multi-Node Local Testnet

If you want to see the multi-node consensus algorithm in action, see [Simulate a
network](https://docs.pezkuwichain.io/tutorials/build-a-blockchain/simulate-network/).

## Template Structure

A Bizinikiwi project such as this consists of a number of components that are
spread across a few directories.

### Node

A blockchain node is an application that allows users to participate in a
blockchain network. Bizinikiwi-based blockchain nodes expose a number of
capabilities:

- Networking: Bizinikiwi nodes use the [`libp2p`](https://libp2p.io/) networking
  stack to allow the nodes in the network to communicate with one another.
- Consensus: Blockchains must have a way to come to
  [consensus](https://docs.pezkuwichain.io/fundamentals/consensus/) on the state of
  the network. Bizinikiwi makes it possible to supply custom consensus engines
  and also ships with several consensus mechanisms that have been built on top
  of [Web3 Foundation
  research](https://research.web3.foundation/PezkuwiChain/protocols/NPoS).
- RPC Server: A remote procedure call (RPC) server is used to interact with
  Bizinikiwi nodes.

There are several files in the `node` directory. Take special note of the
following:

- [`chain_spec.rs`](./node/src/chain_spec.rs): A [chain
  specification](https://docs.pezkuwichain.io/build/chain-spec/) is a source code
  file that defines a Bizinikiwi chain's initial (genesis) state. Chain
  specifications are useful for development and testing, and critical when
  architecting the launch of a production chain. Take note of the
  `development_config` and `testnet_genesis` functions. These functions are
  used to define the genesis state for the local development chain
  configuration. These functions identify some [well-known
  accounts](https://docs.pezkuwichain.io/reference/command-line-tools/pez_subkey/) and
  use them to configure the blockchain's initial state.
- [`service.rs`](./node/src/service.rs): This file defines the node
  implementation. Take note of the libraries that this file imports and the
  names of the functions it invokes. In particular, there are references to
  consensus-related topics, such as the [block finalization and
  forks](https://docs.pezkuwichain.io/fundamentals/consensus/#finalization-and-forks)
  and other [consensus
  mechanisms](https://docs.pezkuwichain.io/fundamentals/consensus/#default-consensus-models)
  such as Aura for block authoring and GRANDPA for finality.


### Runtime

In Bizinikiwi, the terms "runtime" and "state transition function" are analogous.
Both terms refer to the core logic of the blockchain that is responsible for
validating blocks and executing the state changes they define. The Bizinikiwi
project in this repository uses
[FRAME](https://docs.pezkuwichain.io/learn/runtime-development/#frame) to construct
a blockchain runtime. FRAME allows runtime developers to declare domain-specific
logic in modules called "pallets". At the heart of FRAME is a helpful [macro
language](https://docs.pezkuwichain.io/reference/frame-macros/) that makes it easy
to create pallets and flexibly compose them to create blockchains that can
address [a variety of needs](https://bizinikiwi.pezkuwichain.io/ecosystem/projects/).

Review the [FRAME runtime implementation](./runtime/src/lib.rs) included in this
template and note the following:

- This file configures several pallets to include in the runtime. Each pezpallet
  configuration is defined by a code block that begins with `impl
  $PALLET_NAME::Config for Runtime`.
- The pallets are composed into a single runtime by way of the
  [#[runtime]](https://docs.pezkuwichain.io/sdk/master/frame_support/attr.runtime.html)
  macro, which is part of the [core FRAME pezpallet
  library](https://docs.pezkuwichain.io/reference/frame-pallets/#system-pallets).

### Pallets

The runtime in this project is constructed using many FRAME pallets that ship
with [the Bizinikiwi
repository](https://github.com/pezkuwichain/pezkuwi-sdk/tree/main/bizinikiwi/frame) and a
template pezpallet that is [defined in the
`pallets`](./pallets/template/src/lib.rs) directory.

A FRAME pezpallet is comprised of a number of blockchain primitives, including:

- Storage: FRAME defines a rich set of powerful [storage
  abstractions](https://docs.pezkuwichain.io/build/runtime-storage/) that makes it
  easy to use Bizinikiwi's efficient key-value database to manage the evolving
  state of a blockchain.
- Dispatchables: FRAME pallets define special types of functions that can be
  invoked (dispatched) from outside of the runtime in order to update its state.
- Events: Bizinikiwi uses
  [events](https://docs.pezkuwichain.io/build/events-and-errors/) to notify users
  of significant state changes.
- Errors: When a dispatchable fails, it returns an error.

Each pezpallet has its own `Config` trait which serves as a configuration interface
to generically define the types and parameters it depends on.

## Alternatives Installations

Instead of installing dependencies and building this source directly, consider
the following alternatives.

### Nix

Install [nix](https://nixos.org/) and
[nix-direnv](https://github.com/nix-community/nix-direnv) for a fully
plug-and-play experience for setting up the development environment. To get all
the correct dependencies, activate direnv `direnv allow`.

### Docker

Please follow the [Bizinikiwi Docker instructions
here](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/bizinikiwi/docker/README.md) to
build the Docker container with the Bizinikiwi Node Template binary.
