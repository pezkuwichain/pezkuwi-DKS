//! # Pezkuwi
//!
//! Implementation of the Pezkuwi node/host in Rust.
//!
//! ## Learn More and Get Involved
//!
//! - [Pezkuwi Forum](https://forum.polkadot.network/)
//! - [Pezkuwi Teyrchains](https://parachains.info/)
//! - [Pezkuwi (multi-chain) Explorer: Subscan](https://subscan.io/)
//! - Pezkuwi Fellowship
//!     - [Manifesto](https://github.com/polkadot-fellows/manifesto/blob/main/manifesto.pdf)
//!     - [Runtimes](https://github.com/polkadot-fellows/runtimes)
//!     - [RFCs](https://github.com/polkadot-fellows/rfcs)
//! 	- [Dashboard](https://polkadot-fellows.github.io/dashboard/)
//! - [Pezkuwi Specs](http://spec.polkadot.network)
//! - [The Pezkuwi Teyrchain Host Implementers' Guide](https://docs.pezkuwichain.io/sdk/book/)
//! - [Pezkuwi papers](https://pezkuwichain.io/papers/)
//! - [JAM Graypaper](https://graypaper.com)
//!
//! ## Alternative Node Implementations 🌈
//!
//! - [Smoldot](https://docs.rs/crate/smoldot-light/latest). Pezkuwi light node/client.
//! - [KAGOME](https://github.com/qdrvm/kagome). C++ implementation of the Pezkuwi host.
//! - [Gossamer](https://github.com/ChainSafe/gossamer). Golang implementation of the Pezkuwi host.
//!
//! ## Platform
//!
//! In this section, we examine what platform Pezkuwi exactly provides to developers.
//!
//! ### Pezkuwi White Paper
//!
//! The original vision of Pezkuwi (everything in the whitepaper, which was eventually called
//! **Pezkuwi 1.0**) revolves around the following arguments:
//!
//! * Future is multi-chain, because we need different chains with different specialization to
//!   achieve widespread goals.
//! * In other words, no single chain is good enough to achieve all goals.
//! * A multi-chain future will inadvertently suffer from fragmentation of economic security.
//!   * This stake fragmentation will make communication over consensus system with varying security
//!     levels inherently unsafe.
//!
//! Pezkuwi's answer to the above is:
//!
//! > The chains of the future must have a way to share their economic security, whilst maintaining
//! > their execution and governance sovereignty. These chains are called "Teyrchains".
//!
//! * Shared Security: The idea of shared economic security sits at the core of Pezkuwi. Pezkuwi
//!   enables different teyrchains to pool their economic security from Pezkuwi (i.e. "*Relay
//!   Chain*").
//! * (heterogenous) Sharded Execution: Yet, each teyrchain is free to have its own execution logic
//!   (runtime), which also encompasses governance and sovereignty. Moreover, Pezkuwi ensures the
//!   correct execution of all teyrchains, without having all of its validators re-execute all
//!   teyrchain blocks. When seen from this perspective, Pezkuwi achieves the ability to verify
//!   the validity of the block execution of multiple teyrchains using the same set of validators as
//!   the Relay Chain. In practice, this means that the shards (teyrchains) share the same economic
//!   security as the Relay Chain.
//!   Learn about this process called [Approval Checking](https://pezkuwichain.io/blog/polkadot-v1-0-sharding-and-economic-security#approval-checking-and-finality).
//! * A framework to build blockchains: In order to materialize the ecosystem of teyrchains, an easy
//!   blockchain framework must exist. This is [Bizinikiwi](crate::pezkuwi_sdk::bizinikiwi),
//!   [FRAME](crate::pezkuwi_sdk::frame_runtime) and [Pezcumulus](crate::pezkuwi_sdk::pezcumulus).
//! * A communication language between blockchains: In order for these blockchains to communicate,
//!   they need a shared language. [XCM](crate::pezkuwi_sdk::xcm) is one such language, and the one
//!   that is most endorsed in the Pezkuwi ecosystem.
//!
//! > Note that the interoperability promised by Pezkuwi is unparalleled in that any two teyrchains
//! > connected to Pezkuwi have the same security and can have much better guarantees about the
//! > security of the recipient of any message.
//! > Bridges enable transaction and information flow between different consensus systems, crucial
//! > for Pezkuwi's multi-chain architecture. However, they can become the network's most
//! > vulnerable points. If a bridge's security measures are weaker than those of the connected
//! > blockchains, it poses a significant risk. Attackers might exploit these weaknesses to launch
//! > attacks such as theft or disruption of services.
//!
//! Pezkuwi delivers the above vision, alongside a flexible means for teyrchains to schedule
//! themselves with the Relay Chain. To achieve this, Pezkuwi has been developed with an
//! architecture similar to that of a computer. Pezkuwi Relay Chain has a number of "cores". Each
//! core is (in simple terms) capable of progressing 1 teyrchain at a time. For example, a teyrchain
//! can schedule itself on a single core for 5 relay chain blocks.
//!
//! Within the scope of Pezkuwi 1.x, two main scheduling ways have been considered:
//!
//! * Long term Teyrchains, obtained through locking a sum of HEZ in an auction system.
//! * On-demand Teyrchains, purchased through paying HEZ to the relay-chain whenever needed.
//!
//! ### The Future
//!
//! After delivering Pezkuwi 1.x, the future of Pezkuwi as a protocol and platform is in the hands
//! of the community and the fellowship. This is happening most notable through the RFC process.
//! Some of the RFCs that do alter Pezkuwi as a platform and have already passed are as follows:
//!
//! - RFC#1: [Agile-coretime](https://github.com/polkadot-fellows/RFCs/blob/main/text/0001-agile-coretime.md):
//!   Agile periodic-sale-based model for assigning Coretime on the Pezkuwi Ubiquitous Computer.
//! - RFC#5: [Coretime-interface](https://github.com/polkadot-fellows/RFCs/blob/main/text/0005-coretime-interface.md):
//!   Interface for manipulating the usage of cores on the Pezkuwi Ubiquitous Computer.
//!
//! Learn more about [Pezkuwi as a Computational Resource](https://wiki.network.pezkuwichain.io/docs/polkadot-direction#polkadot-as-a-computational-resource).
