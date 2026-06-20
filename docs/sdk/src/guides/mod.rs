//! # Pezkuwi SDK Docs Guides
//!
//! This crate contains a collection of guides that are foundational to the developers of
//! Pezkuwi SDK. They are common user-journeys that are traversed in the Pezkuwi ecosystem.
//!
//! The main user-journey covered by these guides is:
//!
//! * [`your_first_pallet`], where you learn what a FRAME pezpallet is, and write your first
//!   application logic.
//! * [`your_first_runtime`], where you learn how to compile your pallets into a WASM runtime.
//! * [`your_first_node`], where you learn how to run the said runtime in a node.
//!
//! > By this step, you have already launched a full Pezkuwi-SDK-based blockchain!
//!
//! Once done, feel free to step up into one of our templates: [`crate::pezkuwi_sdk::templates`].
//!
//! [`your_first_pallet`]: crate::guides::your_first_pallet
//! [`your_first_runtime`]: crate::guides::your_first_runtime
//! [`your_first_node`]: crate::guides::your_first_node
//!
//! Other guides are related to other miscellaneous topics and are listed as modules below.

/// Write your first simple pezpallet, learning the most most basic features of FRAME along the way.
pub mod your_first_pallet;

/// Write your first real [runtime](`crate::reference_docs::wasm_meta_protocol`),
/// compiling it to [WASM](crate::pezkuwi_sdk::bizinikiwi#wasm-build).
pub mod your_first_runtime;

/// Running the given runtime with a node. No specific consensus mechanism is used at this stage.
pub mod your_first_node;

/// How to enhance a given runtime and node to be pezcumulus-enabled, run it as a teyrchain
/// and connect it to a relay-chain.
// pub mod your_first_teyrchain;

/// How to enable storage weight reclaiming in a teyrchain node and runtime.
pub mod enable_pov_reclaim;

/// How to enable Async Backing on teyrchain projects that started in 2023 or before.
pub mod async_backing_guide;

/// How to enable metadata hash verification in the runtime.
pub mod enable_metadata_hash;

/// How to enable elastic scaling on a teyrchain.
pub mod enable_elastic_scaling;

/// How to parameterize teyrchain forking in relation to relay chain forking.
pub mod handling_teyrchain_forks;
