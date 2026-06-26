#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(flatten)]
	pub run: pezsc_cli::RunCmd,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Subcommand {
	/// Key management cli utilities
	#[command(subcommand)]
	Key(pezsc_cli::KeySubcommand),

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

	/// Remove the whole chain.
	PurgeChain(pezsc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(pezsc_cli::RevertCmd),

	/// Sub-commands concerned with benchmarking.
	#[command(subcommand)]
	Benchmark(pezframe_benchmarking_cli::BenchmarkCmd),

	/// Db meta columns information.
	ChainInfo(pezsc_cli::ChainInfoCmd),
}
