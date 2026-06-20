//! Bizinikiwi Teyrchain Node Template CLI

#![warn(missing_docs)]

use pezkuwi_sdk::*;

mod chain_spec;
mod cli;
mod command;
mod rpc;
mod service;

fn main() -> pezsc_cli::Result<()> {
	command::run()
}
