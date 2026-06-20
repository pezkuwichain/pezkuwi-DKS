//! Bizinikiwi Node Template CLI library.
#![warn(missing_docs)]

mod benchmarking;
mod chain_spec;
mod cli;
mod command;
mod rpc;
mod service;

fn main() -> pezsc_cli::Result<()> {
	command::run()
}
