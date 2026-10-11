//! `paste` 1.x is unmaintained (RUSTSEC-2024-0436). `pastey` is its maintained successor with the
//! same three macros and the same token-pasting rules, so this crate keeps the `paste::` paths
//! every dependent uses and hands the work to `pastey`. It is patched in for crates.io in the
//! workspace manifest; drop it once nothing in the graph asks for `paste` any more.

#![no_std]

pub use pastey::{expr, item, paste};
