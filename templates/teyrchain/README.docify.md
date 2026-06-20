<div align="center">

# Pezkuwi SDK's Teyrchain Template

<img height="70px" alt="Pezkuwi SDK Logo" src="https://github.com/pezkuwichain/pezkuwi-sdk/raw/master/docs/images/Polkadot_Logo_Horizontal_Pink_White.png#gh-dark-mode-only"/>
<img height="70px" alt="Pezkuwi SDK Logo" src="https://github.com/pezkuwichain/pezkuwi-sdk/raw/master/docs/images/Polkadot_Logo_Horizontal_Pink_Black.png#gh-light-mode-only"/>

> This is a template for creating a [teyrchain](https://wiki.network.pezkuwichain.io/docs/learn-parachains) based on Pezkuwi SDK.
>
> This template is automatically updated after releases in the main [Pezkuwi SDK monorepo](https://github.com/pezkuwichain/pezkuwi-sdk).

</div>

## Table of Contents

- [Intro](#intro)

- [Template Structure](#template-structure)

- [Getting Started](#getting-started)

- [Starting a Development Chain](#starting-a-development-chain)

  - [Omni Node](#omni-node-prerequisites)
  - [Zombienet setup with Omni Node](#zombienet-setup-with-omni-node)
  - [Teyrchain Template Node](#teyrchain-template-node)
  - [Connect with the Pezkuwi-JS Apps Front-End](#connect-with-the-pezkuwi-js-apps-front-end)
  - [Takeaways](#takeaways)

- [Runtime development](#runtime-development)
- [Contributing](#contributing)
- [Getting Help](#getting-help)

## Intro

- ⏫ This template provides a starting point to build a [teyrchain](https://wiki.network.pezkuwichain.io/docs/learn-parachains).

- ☁️ It is based on the
  [Pezcumulus](https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/polkadot_sdk/pezcumulus/index.html) framework.

- 🔧 Its runtime is configured with a single custom pezpallet as a starting point, and a handful of ready-made pallets
  such as a [Balances pezpallet](https://docs.pezkuwichain.io/sdk/master/pallet_balances/index.html).

- 👉 Learn more about teyrchains in the [Pezkuwi Wiki](https://wiki.network.pezkuwichain.io/docs/learn-teyrchains)

## Template Structure

A Pezkuwi SDK based project such as this one consists of:

- 🧮 the [Runtime](./runtime/README.md) - the core logic of the teyrchain.
- 🎨 the [Pallets](./pallets/README.md) - from which the runtime is constructed.
- 💿 a [Node](./node/README.md) - the binary application, not part of the project default-members list and not compiled unless
  building the project with `--workspace` flag, which builds all workspace members, and is an alternative to
  [Omni Node](https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/reference_docs/omni_node/index.html).

## Getting Started

- 🦀 The template is using the Rust language.

- 👉 Check the
  [Rust installation instructions](https://www.rust-lang.org/tools/install) for your system.

- 🛠️ Depending on your operating system and Rust version, there might be additional
  packages required to compile this template - please take note of the Rust compiler output.

Fetch teyrchain template code:

```sh
git clone https://github.com/pezkuwichain/pezkuwi-sdk-teyrchain-template.git teyrchain-template

cd teyrchain-template
```

## Starting a Development Chain

The teyrchain template relies on a hardcoded teyrchain id which is defined in the runtime code
and referenced throughout the contents of this file as `{{TEYRCHAIN_ID}}`. Please replace
any command or file referencing this placeholder with the value of the `TEYRCHAIN_ID` constant:

<!-- docify::embed!("runtime/src/genesis_config_presets.rs", TEYRCHAIN_ID)-->

### Omni Node Prerequisites

[Omni Node](https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/reference_docs/omni_node/index.html) can
be used to run the teyrchain template's runtime. `pezkuwi-omni-node` binary crate usage is described at a high-level
[on crates.io](https://crates.io/crates/polkadot-omni-node).

#### Install `pezkuwi-omni-node`

```sh
cargo install pezkuwi-omni-node
```

> For more advanced options, please see the installation section at [`crates.io/omni-node`](https://crates.io/crates/polkadot-omni-node).

#### Build `teyrchain-template-runtime`

```sh
cargo build --profile production
```

#### Install `pezstaging-chain-spec-builder`

```sh
cargo install pezstaging-chain-spec-builder
```

> For more advanced options, please see the installation section at [`crates.io/pezstaging-chain-spec-builder`](https://crates.io/crates/pezstaging-chain-spec-builder).

#### Use `chain-spec-builder` to generate the `chain_spec.json` file

```sh
chain-spec-builder create --relay-chain "pezkuwichain-local" --runtime \
    target/release/wbuild/teyrchain-template-runtime/teyrchain_template_runtime.wasm named-preset development
```

**Note**: the `relay-chain` flag is required by Omni Node. The `relay-chain` value is set in accordance
with the relay chain ID where this instantiation of teyrchain-template will connect to.

#### Run Omni Node

Start Omni Node with the generated chain spec. We'll start it in development mode (without a relay chain config), producing
and finalizing blocks based on manual seal, configured below to seal a block with each second.

```bash
pezkuwi-omni-node --chain <path/to/chain_spec.json> --dev --dev-block-time 1000
```

However, such a setup is not close to what would run in production, and for that we need to setup a local
relay chain network that will help with the block finalization. In this guide we'll setup a local relay chain
as well. We'll not do it manually, by starting one node at a time, but we'll use [zombienet](https://paritytech.github.io/zombienet/intro.html).

Follow through the next section for more details on how to do it.

### Zombienet setup with Omni Node

Assuming we continue from the last step of the previous section, we have a chain spec and we need to setup a relay chain.
We can install `zombienet` as described in the [Zombienet installation guide](https://paritytech.github.io/zombienet/install.html#installation), and
`zombienet-omni-node.toml` contains the network specification we want to start.

#### Relay chain prerequisites

Download the `pezkuwi` (and the accompanying `pezkuwi-prepare-worker` and `pezkuwi-execute-worker`) binaries from
[Pezkuwi SDK releases](https://github.com/pezkuwichain/pezkuwi-sdk/releases). Then expose them on `PATH` like so:

```sh
export PATH="$PATH:<path/to/binaries>"
```

#### Update `zombienet-omni-node.toml` with a valid chain spec path

To simplify the process of using the teyrchain-template with zombienet and Omni Node, we've added a pre-configured
development chain spec (dev_chain_spec.json) to the teyrchain template. The zombienet-omni-node.toml file of this
template points to it, but you can update it to an updated chain spec generated on your machine. To generate a
chain spec refer to [pezstaging-chain-spec-builder](https://crates.io/crates/pezstaging-chain-spec-builder)

Then make the changes in the network specification like so:

```toml
# ...
[[teyrchains]]
id = "<TEYRCHAIN_ID>"
chain_spec_path = "<TO BE UPDATED WITH A VALID PATH>"
# ...
```

#### Start the network

```sh
zombienet --provider native spawn zombienet-omni-node.toml
```

### Teyrchain Template Node

As mentioned in the `Template Structure` section, the `node` crate is optionally compiled and it is an alternative
to `Omni Node`. Similarly, it requires setting up a relay chain, and we'll use `zombienet` once more.

#### Install the `teyrchain-template-node`

```sh
cargo install --path node --locked
```

#### Setup and start the network

For setup, please consider the [Zombienet installation guide](https://paritytech.github.io/zombienet/install.html#installation)
and [relay chain prerequisites](#relay-chain-prerequisites).

We're left just with starting the network:

```sh
zombienet --provider native spawn zombienet.toml
```

### Connect with the Pezkuwi-JS Apps Front-End

- 🌐 You can interact with your local node using the
  hosted version of the Pezkuwi/Bizinikiwi Portal:
  [relay chain](https://pezkuwichain.io/#/explorer?rpc=ws://localhost:9944)
  and [teyrchain](https://pezkuwichain.io/#/explorer?rpc=ws://localhost:9988).

- 🪐 A hosted version is also
  available on [IPFS](https://dotapps.io/).

- 🧑‍🔧 You can also find the source code and instructions for hosting your own instance in the
  [`pezkuwi-js/apps`](https://github.com/polkadot-js/apps) repository.

### Takeaways

Development teyrchains:

- 🔗 Connect to relay chains, and we showcased how to connect to a local one.
- 🧹 Do not persist the state.
- 💰 Are preconfigured with a genesis state that includes several prefunded development accounts.
- 🧑‍⚖️ Development accounts are used as validators, collators, and `sudo` accounts.

## Runtime development

We recommend using [`chopsticks`](https://github.com/AcalaNetwork/chopsticks) when the focus is more on the runtime
development and `OmniNode` is enough as is.

### Install chopsticks

To use `chopsticks`, please install the latest version according to the installation [guide](https://github.com/AcalaNetwork/chopsticks?tab=readme-ov-file#install).

### Build a raw chain spec

Build the `teyrchain-template-runtime` as mentioned before in this guide and use `chain-spec-builder`
again but this time by passing `--raw-storage` flag:

```sh
chain-spec-builder create --raw-storage --relay-chain "pezkuwichain-local" --runtime \
    target/release/wbuild/teyrchain-template-runtime/teyrchain_template_runtime.wasm named-preset development
```

### Start `chopsticks` with the chain spec

```sh
npx @acala-network/chopsticks@latest --chain-spec <path/to/chain_spec.json>
```

### Alternatives

`OmniNode` can be still used for runtime development if using the `--dev` flag, while `teyrchain-template-node` doesn't
support it at this moment. It can still be used to test a runtime in a full setup where it is started alongside a
relay chain network (see [Teyrchain Template node](#teyrchain-template-node) setup).

## Contributing

- 🔄 This template is automatically updated after releases in the main [Pezkuwi SDK monorepo](https://github.com/pezkuwichain/pezkuwi-sdk).

- ➡️ Any pull requests should be directed to this [source](https://github.com/pezkuwichain/pezkuwi-sdk/tree/main/templates/teyrchain).

- 😇 Please refer to the monorepo's
  [contribution guidelines](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/docs/contributor/CONTRIBUTING.md) and
  [Code of Conduct](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/docs/contributor/CODE_OF_CONDUCT.md).

## Getting Help

- 🧑‍🏫 To learn about Pezkuwi in general, [docs.Pezkuwi.com](https://docs.pezkuwichain.io/) website is a good starting point.

- 🧑‍🔧 For technical introduction, see [the Pezkuwi SDK documentation](https://github.com/pezkuwichain/pezkuwi-sdk#-documentation).

- 👥 Additionally, there are [GitHub issues](https://github.com/pezkuwichain/pezkuwi-sdk/issues) and
  [Bizinikiwi StackExchange](https://exchange.pezkuwichain.io/).
- 👥You can also reach out on the [Official Pezkuwi discord server](https://polkadot-discord.w3f.tools/)
- 🧑Reach out on [Telegram](https://t.me/bizinikiwidevs) for more questions and discussions
