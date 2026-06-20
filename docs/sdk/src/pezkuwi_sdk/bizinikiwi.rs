//! # Bizinikiwi
//!
//! Bizinikiwi is a Rust framework for building blockchains in a modular and extensible way. While
//! in itself un-opinionated, it is the main engine behind the Pezkuwi ecosystem.
//!
//! ## Overview, Philosophy
//!
//! Bizinikiwi approaches blockchain development with an acknowledgement of a few self-evident
//! truths:
//!
//! 1. Society and technology evolves.
//! 2. Humans are fallible.
//!
//! This makes the task of designing a correct, safe and long-lasting blockchain system hard.
//!
//! Nonetheless, in strive towards achieving this goal, Bizinikiwi embraces the following:
//!
//! 1. Use of **Rust** as a modern and safe programming language, which limits human error through
//!    various means, most notably memory and type safety.
//! 2. Bizinikiwi is written from the ground-up with a *generic, modular and extensible* design.
//!    This ensures that software components can be easily swapped and upgraded. Examples of this is
//!    multiple consensus mechanisms provided by Bizinikiwi, as listed below.
//! 3. Lastly, the final blockchain system created with the above properties needs to be
//!    upgradeable. In order to achieve this, Bizinikiwi is designed as a meta-protocol, whereby the
//!    application logic of the blockchain (called "Runtime") is encoded as a WASM blob, and is
//!    stored in the state. The rest of the system (called "node") acts as the executor of the WASM
//!    blob.
//!
//! In essence, the meta-protocol of all Bizinikiwi based chains is the "Runtime as WASM blob"
//! accord. This enables the Runtime to become inherently upgradeable, crucially without [forks](https://en.wikipedia.org/wiki/Fork_(blockchain)). The
//! upgrade is merely a matter of the WASM blob being changed in the state, which is, in principle,
//! same as updating an account's balance. Learn more about this in detail in
//! [`crate::reference_docs::wasm_meta_protocol`].
//!
//! > A great analogy for bizinikiwi is the following: Bizinikiwi node is a gaming console, and a
//! > WASM
//! > runtime, possibly created with FRAME is the game being inserted into the console.
//!
//! [`frame`], Bizinikiwi's default runtime development library, takes the above safety practices
//! even further by embracing a declarative programming model whereby correctness is enhanced and
//! the system is highly configurable through parameterization. Learn more about this in
//! [`crate::reference_docs::trait_based_programming`].
//!
//! ## How to Get Started
//!
//! Bizinikiwi offers different options at the spectrum of technical freedom <-> development ease.
//!
//! * The easiest way to use Bizinikiwi is to use one of the templates (some of which listed at
//!   [`crate::pezkuwi_sdk::templates`]) and only tweak the parameters of the runtime or node. This
//!   allows you to launch a blockchain in minutes, but is limited in technical freedom.
//! * Next, most developers wish to develop their custom runtime modules, for which the de-facto way
//! is [`frame`](crate::pezkuwi_sdk::frame_runtime).
//! * Finally, Bizinikiwi is highly configurable at the node side as well, but this is the most
//!   technically demanding.
//!
//! > A notable Bizinikiwi-based blockchain that has built both custom FRAME pallets and custom
//! > node-side components is <https://github.com/Cardinal-Cryptography/aleph-node>.
#![doc = simple_mermaid::mermaid!("../../../mermaid/bizinikiwi_dev.mmd")]
//!
//! ## Structure
//!
//! Bizinikiwi contains a large number of crates, therefore it is useful to have an overview of what
//! they are, and how they are organized. In broad terms, these crates are divided into three
//! categories:
//!
//! * `sc-*` (short for *Bizinikiwi-client*) crates, located under `./client` folder. These are all
//!   the crates that lead to the node software. Notable examples are [`pezsc_network`], various
//!   consensus crates, RPC ([`pezsc_rpc_api`]) and database ([`pezsc_client_db`]), all of which are
//!   expected to reside in the node side.
//! * `sp-*` (short for *bizinikiwi-primitives*) crates, located under `./primitives` folder. These
//!   are crates that facilitate both the node and the runtime, but are not opinionated about what
//!   framework is using for building the runtime. Notable examples are [`pezsp_api`] and
//!   [`pezsp_io`], which form the communication bridge between the node and runtime.
//! * `pezpallet-*` and `frame-*` crates, located under `./frame` folder. These are the crates
//!   related to FRAME. See [`frame`] for more information.
//!
//! ### WASM Build
//!
//! Many of the Bizinikiwi crates, such as entire `sp-*`, need to compile to both WASM (when a WASM
//! runtime is being generated) and native (for example, when testing). To achieve this, Bizinikiwi
//! follows the convention of the Rust community, and uses a `feature = "std"` to signify that a
//!  crate is being built with the standard library, and is built for native. Otherwise, it is built
//!  for `no_std`.
//!
//! This can be summarized in `#![cfg_attr(not(feature = "std"), no_std)]`, which you can often find
//! in any Bizinikiwi-based runtime.
//!
//! Bizinikiwi-based runtimes use [`bizinikiwi_wasm_builder`] in their `build.rs` to automatically
//! build their WASM files as a part of normal build command (e.g. `cargo build`). Once built, the
//! wasm file is placed in `./target/{debug|release}/wbuild/{runtime_name}/{runtime_name}.wasm`.
//!
//! In order to ensure that the WASM build is **deterministic**, the [Bizinikiwi Runtime Toolbox (srtool)](https://github.com/paritytech/srtool) can be used.
//!
//! ### Anatomy of a Binary Crate
//!
//! From the above, [`node_cli`]/[`pez_kitchensink_runtime`] and `node-template` are essentially
//! blueprints of a Bizinikiwi-based project, as the name of the latter is implying. Each
//! Bizinikiwi-based project typically contains the following:
//!
//! * Under `./runtime`, a `./runtime/src/lib.rs` which is the top level runtime amalgamator file.
//!   This file typically contains the [`pezframe::runtime::prelude::construct_runtime`] and
//!   [`pezframe::runtime::prelude::impl_runtime_apis`] macro calls, which is the final definition of a
//!   runtime.
//!
//! * Under `./node`, a `main.rs`, which is the starting point, and a `./service.rs`, which contains
//!   all the node side components. Skimming this file yields an overview of the networking,
//!   database, consensus and similar node side components.
//!
//! > The above two are conventions, not rules.
//!
//! > See <https://github.com/pezkuwichain/pezkuwi-sdk/issues/241> for an update on how the node side
//! > components are being amalgamated.
//!
//! ## Teyrchain?
//!
//! As noted above, Bizinikiwi is the main engine behind the Pezkuwi ecosystem. One of the ways
//! through which Pezkuwi can be utilized is by building "teyrchains", blockchains that are
//! connected to Pezkuwi's shared security.
//!
//! To build a teyrchain, one could use [Pezcumulus](crate::pezkuwi_sdk::pezcumulus), the library on
//! top of Bizinikiwi, empowering any bizinikiwi-based chain to be a Pezkuwi teyrchain.
//!
//! ## Where To Go Next?
//!
//! Additional noteworthy crates within bizinikiwi:
//!
//! - RPC APIs of a Bizinikiwi node: [`pezsc_rpc_api`]/[`pezsc_rpc`]
//! - CLI Options of a Bizinikiwi node: [`pezsc_cli`]
//! - All of the consensus related crates provided by Bizinikiwi:
//!     - [`pezsc_consensus_aura`]
//!     - [`pezsc_consensus_babe`]
//!     - [`pezsc_consensus_grandpa`]
//!     - [`pezsc_consensus_beefy`] (TODO: @adrian, add some high level docs <https://github.com/pezkuwichain/pezkuwi-sdk/issues/305>)
//!     - [`pezsc_consensus_manual_seal`]
//!     - [`pezsc_consensus_pow`]
//!
//! [`frame`]: crate::pezkuwi_sdk::frame_runtime

#[doc(hidden)]
pub use crate::pezkuwi_sdk;
