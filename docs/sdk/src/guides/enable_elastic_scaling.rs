//! # Enable elastic scaling for a teyrchain
//!
//! <div class="warning">This guide assumes full familiarity with Asynchronous Backing and its
//! terminology, as defined in <a href="https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/guides/async_backing_guide/index.html">the Pezkuwi SDK Docs</a>.
//! </div>
//!
//! ## Quick introduction to Elastic Scaling
//!
//! [Elastic scaling](https://www.parity.io/blog/polkadot-web3-cloud) is a feature that enables teyrchains (rollups) to use multiple cores.
//! Teyrchains can adjust their usage of core resources on the fly to increase TPS and decrease
//! latency.
//!
//! ### When do you need Elastic Scaling?
//!
//! Depending on their use case, applications might have an increased need for the following:
//! - compute (CPU weight)
//! - bandwidth (proof size)
//! - lower latency (block time)
//!
//! ### High throughput (TPS) and lower latency
//!
//! If the main bottleneck is the CPU, then your teyrchain needs to maximize the compute usage of
//! each core while also achieving a lower latency.
//! 3 cores provide the best balance between CPU, bandwidth and latency: up to 6s of execution,
//! 5MB/s of DA bandwidth and fast block time of just 2 seconds.
//!
//! ### High bandwidth
//!
//! Useful for applications that are bottlenecked by bandwidth.
//! By using 6 cores, applications can make use of up to 6s of compute, 10MB/s of bandwidth
//! while also achieving 1 second block times.
//!
//! ### Ultra low latency
//!
//! When latency is the primary requirement, Elastic scaling is currently the only solution. The
//! caveat is the efficiency of core time usage decreases as more cores are used.
//!
//! For example, using 12 cores enables fast transaction confirmations with 500ms blocks and up to
//! 20 MB/s of DA bandwidth.
//!
//! ## Dependencies
//!
//! Prerequisites: Pezkuwi-SDK `2509` or newer.
//!
//! To ensure the security and reliability of your chain when using this feature you need the
//! following:
//! - An omni-node based collator. This has already become the default choice for collators.
//! - UMP signal support.
//! [RFC103](https://github.com/polkadot-fellows/RFCs/blob/main/text/0103-introduce-core-index-commitment.md).
//!   This is mandatory protection against PoV replay attacks.
//! - Enabling the relay parent offset feature. This is required to ensure the teyrchain block times
//!   and transaction in-block confidence are not negatively affected by relay chain forks. Read
//!   [`crate::guides::handling_teyrchain_forks`] for more information.
//! - Block production configuration adjustments.
//!
//! ### Upgrade to Pezkuwi Omni node
//!
//! Your collators need to run `pezkuwi-teyrchain` or `pezkuwi-omni-node` with the `--authoring
//! slot-based` CLI argument.
//! To avoid potential issues and get best performance it is recommeneded to always run the  
//! latest release on all of the collators.
//!
//! Further information about omni-node and how to upgrade is available:
//! - [high level docs](https://docs.pezkuwichain.io/develop/toolkit/parachains/polkadot-omni-node/)
//! - [`crate::reference_docs::omni_node`]
//!
//! ### UMP signals
//!
//! UMP signals are now enabled by default in the `teyrchain-system` pezpallet and are used for
//! elastic scaling. You can find more technical details about UMP signals and their usage for
//! elastic scaling
//! [here](https://github.com/polkadot-fellows/RFCs/blob/main/text/0103-introduce-core-index-commitment.md).
//!
//! ### Enable the relay parent offset feature
//!
//! It is recommended to use an offset of `1`, which is sufficient to eliminate any issues
//! with relay chain forks.
//!
//! Configure the relay parent offset like this:
//! ```ignore
//!     /// Build with an offset of 1 behind the relay chain best block.
//!     const RELAY_PARENT_OFFSET: u32 = 1;
//!
//!     impl pezcumulus_pezpallet_teyrchain_system::Config for Runtime {
//!         // ...
//!         type RelayParentOffset = ConstU32<RELAY_PARENT_OFFSET>;
//!     }
//! ```
//!
//! Implement the runtime API to retrieve the offset on the client side.
//! ```ignore
//!     impl pezcumulus_primitives_core::RelayParentOffsetApi<Block> for Runtime {
//!         fn relay_parent_offset() -> u32 {
//!             RELAY_PARENT_OFFSET
//!         }
//!     }
//! ```
//!
//! ### Block production configuration
//!
//! This configuration directly controls the minimum block time and maximum number of cores
//! the teyrchain can use.
//!
//! Example configuration for a 3 core teyrchain:
//!  ```ignore
//!     /// The upper limit of how many teyrchain blocks are processed by the relay chain per
//!     /// parent. Limits the number of blocks authored per slot. This determines the minimum
//!     /// block time of the teyrchain:
//!     /// `RELAY_CHAIN_SLOT_DURATION_MILLIS/BLOCK_PROCESSING_VELOCITY`
//!     const BLOCK_PROCESSING_VELOCITY: u32 = 3;
//!
//!     /// Maximum number of blocks simultaneously accepted by the Runtime, not yet included
//!     /// into the relay chain.
//!     const UNINCLUDED_SEGMENT_CAPACITY: u32 = (2 + RELAY_PARENT_OFFSET) *
//! BLOCK_PROCESSING_VELOCITY + 1;
//!
//!     /// Relay chain slot duration, in milliseconds.
//!     const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;
//!
//!     type ConsensusHook = pezcumulus_pezpallet_aura_ext::FixedVelocityConsensusHook<
//!         Runtime,
//!         RELAY_CHAIN_SLOT_DURATION_MILLIS,
//!         BLOCK_PROCESSING_VELOCITY,
//!         UNINCLUDED_SEGMENT_CAPACITY,
//!     >;
//!
//!  ```
//!
//! ### Teyrchain Slot Duration
//!
//! A common source of confusion is the correct configuration of the `SlotDuration` that is passed
//! to `pezpallet-aura`.
//! ```ignore
//! impl pezpallet_aura::Config for Runtime {
//!     // ...
//!     type SlotDuration = ConstU64<SLOT_DURATION>;
//! }
//! ```
//!
//! The slot duration determines the length of each author's turn and is decoupled from the block
//! production interval. During their slot, authors are allowed to produce multiple blocks. **The
//! slot duration is required to be at least 6s (same as on the relay chain).**
//!
//! **Configuration recommendations:**
//! - For new teyrchains starting from genesis: use a slot duration of 24 seconds
//! - For existing live teyrchains: leave the slot duration unchanged
//!
//!
//! ## Current limitations
//!
//! ### Maximum execution time per relay chain block.
//!
//! Since teyrchain block authoring is sequential, the next block can only be built after
//! the previous one has been imported.
//! At present, a core allows up to 2 seconds of execution per relay chain block.
//!
//! If we assume a 6s teyrchain slot, and each block takes the full 2 seconds to execute,
//! the teyrchain will not be able to fully utilize the compute resources of all 3 cores.
//!    
//! If the collator hardware is faster, it can author and import full blocks more quickly,
//! making it possible to utilize even more than 3 cores efficiently.
//!
//! #### Why?
//!
//! Within a 6-second teyrchain slot, collators can author multiple teyrchain blocks.
//! Before building the first block in a slot, the new block author must import the last
//! block produced by the previous author.
//! If the import of the last block is not completed before the next relay chain slot starts,
//! the new author will build on its parent (assuming it was imported). This will create a fork
//! which degrades the teyrchain block confidence and block times.
//!
//! This means that, on reference hardware, a teyrchain with a slot time of 6s can
//! effectively utilize up to 4 seconds of execution per relay chain block, because it needs to
//! ensure the next block author has enough time to import the last block.
//! Hardware with higher single-core performance can enable a teyrchain to fully utilize more
//! cores.
//!
//! ### Fixed factor scaling.
//!
//! For true elasticity, a teyrchain needs to acquire more cores when needed in an automated
//! manner. This functionality is not yet available in the SDK, thus acquiring additional
//! on-demand or bulk cores has to be managed externally.
