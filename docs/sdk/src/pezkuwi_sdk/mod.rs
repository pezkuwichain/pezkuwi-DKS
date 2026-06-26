//! # Pezkuwi SDK
//!
//! [Pezkuwi SDK](https://github.com/pezkuwichain/pezkuwi-sdk) provides the main resources needed to
//! start building on the [Pezkuwi network](https://pezkuwichain.io/), a scalable, multi-chain
//! blockchain platform that enables different blockchains to securely interoperate.
//!
//! [![StackExchange](https://img.shields.io/badge/StackExchange-Polkadot%20and%20Bizinikiwi-222222?logo=stackexchange)](https://exchange.pezkuwichain.app/)
//!
//! [![awesomeDot](https://img.shields.io/badge/polkadot-awesome-e6007a?logo=polkadot)](https://github.com/Awsmdot/awesome-dot)
//! [![wiki](https://img.shields.io/badge/polkadot-wiki-e6007a?logo=polkadot)](https://wiki.network.pezkuwichain.io/)
//! [![forum](https://img.shields.io/badge/polkadot-forum-e6007a?logo=polkadot)](https://forum.polkadot.network/)
//!
//! [![RFCs](https://img.shields.io/badge/fellowship-RFCs-e6007a?logo=polkadot)](https://github.com/polkadot-fellows/rfcs)
//! [![Runtime](https://img.shields.io/badge/fellowship-runtimes-e6007a?logo=polkadot)](https://github.com/polkadot-fellows/runtimes)
//! [![Manifesto](https://img.shields.io/badge/fellowship-manifesto-e6007a?logo=polkadot)](https://github.com/polkadot-fellows/manifesto/blob/main/manifesto.pdf)
//!
//! ## Getting Started
//!
//! The primary way to get started with the Pezkuwi SDK is to start writing a FRAME-based runtime.
//! See:
//!
//! * [`pezkuwi`], to understand what is Pezkuwi as a development platform.
//! * [`bizinikiwi`], for an overview of what Bizinikiwi as the main blockchain framework of Pezkuwi
//!   SDK.
//! * [`frame`], to learn about how to write blockchain applications aka. "App Chains".
//! * Continue with the [`pezkuwi_sdk_docs`'s "getting started"](crate#getting-started).
//!
//! ## Components
//!
//! #### Bizinikiwi
//!
//! [![Bizinikiwi-license](https://img.shields.io/badge/License-GPL3%2FApache2.0-blue)](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/bizinikiwi/LICENSE-APACHE2)
//! [![GitHub
//! Repo](https://img.shields.io/badge/github-bizinikiwi-2324CC85)](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/bizinikiwi)
//!
//! [`bizinikiwi`] is the base blockchain framework used to power the Pezkuwi SDK. It is a full
//! toolkit to create sovereign blockchains, including but not limited to those which connect to
//! Pezkuwi as teyrchains.
//!
//! #### FRAME
//!
//! [![Bizinikiwi-license](https://img.shields.io/badge/License-Apache2.0-blue)](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/bizinikiwi/LICENSE-APACHE2)
//! [![GitHub
//! Repo](https://img.shields.io/badge/github-frame-2324CC85)](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/bizinikiwi/pezframe)
//!
//! [`frame`] is the framework used to create Bizinikiwi-based application logic, aka. runtimes.
//! Learn more about the distinction of a runtime and node in
//! [`reference_docs::wasm_meta_protocol`].
//!
//! #### Pezcumulus
//!
//! [![Pezcumulus-license](https://img.shields.io/badge/License-GPL3-blue)](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/pezcumulus/LICENSE)
//! [![GitHub
//! Repo](https://img.shields.io/badge/github-pezcumulus-white)](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/pezcumulus)
//!
//! [`pezcumulus`] transforms FRAME-based runtimes into Pezkuwi-compatible teyrchain runtimes, and
//! Bizinikiwi-based nodes into Pezkuwi/Teyrchain-compatible nodes.
//!
//! #### XCM
//!
//! [![XCM-license](https://img.shields.io/badge/License-GPL3-blue)](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/pezkuwi/LICENSE)
//! [![GitHub
//! Repo](https://img.shields.io/badge/github-XCM-e6007a?logo=polkadot)](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/pezkuwi/xcm)
//!
//! [`xcm`], short for "cross consensus message", is the primary format that is used for
//! communication between teyrchains, but is intended to be extensible to other use cases as well.
//!
//! #### Pezkuwi
//!
//! [![Pezkuwi-license](https://img.shields.io/badge/License-GPL3-blue)](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/pezkuwi/LICENSE)
//! [![GitHub
//! Repo](https://img.shields.io/badge/github-polkadot-e6007a?logo=polkadot)](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/pezkuwi)
//!
//! [`pezkuwi`] is an implementation of a Pezkuwi node in Rust, by `@paritytech`. The Pezkuwi
//! runtimes are located under the
//! [`pezkuwi-fellows/runtimes`](https://github.com/polkadot-fellows/runtimes) repository.
//!
//! ### Binaries
//!
//! The main binaries that are part of the Pezkuwi SDK are:

//! * [`pezkuwi`]: The Pezkuwi relay chain node binary, as noted above.
//! * [`pezkuwi-omni-node`]: A white-labeled teyrchain collator node. See more in
//!   [`crate::reference_docs::omni_node`].
//! * [`pezkuwi-teyrchain-bin`]: The collator node used to run collators for all Pezkuwi system
//!   teyrchains.
//! * [`frame-omni-bencher`]: a benchmarking tool for FRAME-based runtimes. Nodes typically contain
//!   a
//!  `benchmark` subcommand that does the same.
//! * [`chain_spec_builder`]: Utility to build chain-specs Nodes  typically contain a `build-spec`
//!   subcommand that does the same.
//! * [`pez_subkey`]: Bizinikiwi's key management utility.
//! * [`bizinikiwi-node`](node_cli) is an extensive bizinikiwi node that contains the superset of
//!   all runtime and node side features. The corresponding runtime, called
//!   [`pez_kitchensink_runtime`] contains all of the modules that are provided with `FRAME`. This
//!   node and runtime is only used for testing and demonstration.
//!
//! ### Summary
//!
//! The following diagram summarizes how some of the components of Pezkuwi SDK work together:
#![doc = simple_mermaid::mermaid!("../../../mermaid/pezkuwi_sdk_bizinikiwi.mmd")]
//!
//! A Bizinikiwi-based chain is a blockchain composed of a runtime and a node. As noted above, the
//! runtime is the application logic of the blockchain, and the node is everything else.
//! See [`reference_docs::wasm_meta_protocol`] for an in-depth explanation of this. The
//! former is built with [`frame`], and the latter is built with rest of Bizinikiwi.
//!
//! > You can think of a Bizinikiwi-based chain as a white-labeled blockchain.
#![doc = simple_mermaid::mermaid!("../../../mermaid/pezkuwi_sdk_pezkuwi.mmd")]
//! Pezkuwi is itself a Bizinikiwi-based chain, composed of the exact same two components. It has
//! specialized logic in both the node and the runtime side, but it is not "special" in any way.
//!
//! A teyrchain is a "special" Bizinikiwi-based chain, whereby both the node and the runtime
//! components have became "Pezkuwi-aware" using Pezcumulus.
#![doc = simple_mermaid::mermaid!("../../../mermaid/pezkuwi_sdk_teyrchain.mmd")]
//!
//! ## Notable Upstream Crates
//!
//! - [`parity-scale-codec`](https://github.com/pezkuwichain/parity-scale-codec)
//! - [`parity-db`](https://github.com/pezkuwichain/parity-db)
//! - [`trie`](https://github.com/paritytech/trie)
//! - [`parity-common`](https://github.com/paritytech/parity-common)
//!
//! ## Trophy Section: Notable Downstream Projects
//!
//! A list of projects and tools in the blockchain ecosystem that one way or another use parts of
//! the Pezkuwi SDK:
//!
//! * [Avail](https://github.com/availproject/avail)
//! * [Cardano Partner Chains](https://iohk.io/en/blog/posts/2023/11/03/partner-chains-are-coming-to-cardano/)
//! * [Starknet's Madara Sequencer](https://github.com/keep-starknet-strange/madara)
//! * [Polymesh](https://polymesh.network/)
//!
//! [`bizinikiwi`]: crate::pezkuwi_sdk::bizinikiwi
//! [`frame`]: crate::pezkuwi_sdk::frame_runtime
//! [`pezcumulus`]: crate::pezkuwi_sdk::pezcumulus
//! [`pezkuwi`]: crate::pezkuwi_sdk::pezkuwi
//! [`xcm`]: crate::pezkuwi_sdk::xcm
//! [`frame-omni-bencher`]: https://crates.io/crates/frame-omni-bencher
//! [`pezkuwi-teyrchain-bin`]: https://crates.io/crates/polkadot-parachain-bin
//! [`pezkuwi-omni-node`]: https://crates.io/crates/polkadot-omni-node

/// Learn about Bizinikiwi, the main blockchain framework used in the Pezkuwi ecosystem.
pub mod bizinikiwi;
/// Learn about FRAME, the framework used to build Bizinikiwi runtimes.
pub mod frame_runtime;
/// Learn about Pezcumulus, the framework that transforms [`bizinikiwi`]-based chains into
/// [`pezkuwi`]-enabled teyrchains.
pub mod pezcumulus;
/// Learn about Pezkuwi as a platform.
pub mod pezkuwi;
/// Learn about different ways through which smart contracts can be utilized on top of Bizinikiwi,
/// and in the Pezkuwi ecosystem.
pub mod smart_contracts;
/// Index of all the templates that can act as first scaffold for a new project.
pub mod templates;
/// Learn about XCM, the de-facto communication language between different consensus systems.
pub mod xcm;
