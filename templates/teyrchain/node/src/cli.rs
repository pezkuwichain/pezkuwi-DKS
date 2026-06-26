use pezkuwi_sdk::*;
use std::path::PathBuf;

/// Sub-commands supported by the collator.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Build a chain specification.
	/// DEPRECATED: `build-spec` command will be removed after 1/04/2026. Use `export-chain-spec`
	/// command instead.
	#[deprecated(
		note = "build-spec command will be removed after 1/04/2026. Use export-chain-spec command instead"
	)]
	BuildSpec(pezsc_cli::BuildSpecCmd),

	/// Export the chain specification.
	ExportChainSpec(pezsc_cli::ExportChainSpecCmd),

	/// Validate blocks.
	CheckBlock(pezsc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(pezsc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(pezsc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(pezsc_cli::ImportBlocksCmd),

	/// Revert the chain to a previous state.
	Revert(pezsc_cli::RevertCmd),

	/// Remove the whole chain.
	PurgeChain(pezcumulus_client_cli::PurgeChainCmd),

	/// Export the genesis head data of the teyrchain.
	///
	/// Head data is the encoded block header.
	#[command(alias = "export-genesis-state")]
	ExportGenesisHead(pezcumulus_client_cli::ExportGenesisHeadCommand),

	/// Export the genesis wasm of the teyrchain.
	ExportGenesisWasm(pezcumulus_client_cli::ExportGenesisWasmCommand),

	/// Sub-commands concerned with benchmarking.
	/// The pezpallet benchmarking moved to the `pezpallet` sub-command.
	#[command(subcommand)]
	Benchmark(pezframe_benchmarking_cli::BenchmarkCmd),
}

const AFTER_HELP_EXAMPLE: &str = color_print::cstr!(
	r#"<bold><underline>Examples:</></>
   <bold>teyrchain-template-node build-spec --disable-default-bootnode > plain-teyrchain-chainspec.json</>
           Export a chainspec for a local testnet in json format.
   <bold>teyrchain-template-node --chain plain-teyrchain-chainspec.json --tmp -- --chain pezkuwichain-local</>
           Launch a full node with chain specification loaded from plain-teyrchain-chainspec.json.
   <bold>teyrchain-template-node</>
           Launch a full node with default teyrchain <italic>local-testnet</> and relay chain <italic>pezkuwichain-local</>.
   <bold>teyrchain-template-node --collator</>
           Launch a collator with default teyrchain <italic>local-testnet</> and relay chain <italic>pezkuwichain-local</>.
 "#
);
#[derive(Debug, clap::Parser)]
#[command(
	propagate_version = true,
	args_conflicts_with_subcommands = true,
	subcommand_negates_reqs = true
)]
#[clap(after_help = AFTER_HELP_EXAMPLE)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[command(flatten)]
	pub run: pezcumulus_client_cli::RunCmd,

	/// Disable automatic hardware benchmarks.
	///
	/// By default these benchmarks are automatically ran at startup and measure
	/// the CPU speed, the memory bandwidth and the disk speed.
	///
	/// The results are then printed out in the logs, and also sent as part of
	/// telemetry, if telemetry is enabled.
	#[arg(long)]
	pub no_hardware_benchmarks: bool,

	/// Relay chain arguments
	#[arg(raw = true)]
	pub relay_chain_args: Vec<String>,
}

#[derive(Debug)]
pub struct RelayChainCli {
	/// The actual relay chain cli object.
	pub base: pezkuwi_cli::RunCmd,

	/// Optional chain id that should be passed to the relay chain.
	pub chain_id: Option<String>,

	/// The base path that should be used by the relay chain.
	pub base_path: Option<PathBuf>,
}

impl RelayChainCli {
	/// Parse the relay chain CLI parameters using the para chain `Configuration`.
	pub fn new<'a>(
		para_config: &pezsc_service::Configuration,
		relay_chain_args: impl Iterator<Item = &'a String>,
	) -> Self {
		let extension = crate::chain_spec::Extensions::try_get(&*para_config.chain_spec);
		let chain_id = extension.map(|e| e.relay_chain.clone());
		let base_path = para_config.base_path.path().join("pezkuwi");
		Self {
			base_path: Some(base_path),
			chain_id,
			base: clap::Parser::parse_from(relay_chain_args),
		}
	}
}
