//! Pezkuwi Zombienet CLI - Network orchestration tool for Pezkuwi blockchain testing
//!
//! This CLI provides commands to spawn, manage, and test Pezkuwi blockchain networks
//! using the pezkuwi-zombienet-sdk.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use pezkuwi_zombienet_sdk::{NetworkConfig, NetworkConfigExt};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Pezkuwi Zombienet CLI - Network orchestration for Pezkuwi blockchain testing
#[derive(Parser, Debug)]
#[command(name = "pezkuwi-zombienet")]
#[command(author = "Kurdistan Tech Institute")]
#[command(version)]
#[command(about = "Network orchestration tool for Pezkuwi blockchain testing", long_about = None)]
struct Cli {
	/// Enable verbose output
	#[arg(short, long, global = true)]
	verbose: bool,

	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
	/// Spawn a new network from a configuration file
	Spawn {
		/// Path to the TOML configuration file
		#[arg(value_name = "CONFIG_FILE")]
		config: PathBuf,

		/// Provider to use for spawning the network
		#[arg(short, long, value_enum, default_value = "native")]
		provider: Provider,
	},
}

#[derive(ValueEnum, Clone, Debug)]
enum Provider {
	/// Run nodes directly as native processes
	Native,
	/// Run nodes in Docker containers
	Docker,
	/// Run nodes in Kubernetes
	K8s,
}

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();

	// Set up tracing/logging
	let level = if cli.verbose { Level::DEBUG } else { Level::INFO };
	let subscriber = FmtSubscriber::builder()
		.with_max_level(level)
		.with_target(false)
		.with_thread_ids(false)
		.with_file(false)
		.with_line_number(false)
		.finish();
	tracing::subscriber::set_global_default(subscriber)
		.context("Failed to set tracing subscriber")?;

	match cli.command {
		Commands::Spawn { config, provider } => {
			spawn_network(config, provider).await?;
		},
	}

	Ok(())
}

async fn spawn_network(config_path: PathBuf, provider: Provider) -> Result<()> {
	let config_str = config_path.to_str().context("Invalid config path")?;

	info!("Loading network configuration from: {}", config_str);

	let network_config = NetworkConfig::load_from_toml(config_str)
		.context("Failed to load network configuration")?;

	info!("Network configuration loaded successfully");
	info!("Relay chain: {}", network_config.relaychain().chain().as_str());
	info!("Teyrchains: {}", network_config.teyrchains().len());

	info!("Spawning network with provider: {:?}", provider);

	let network = match provider {
		Provider::Native => {
			info!("Using native provider (direct process spawning)");
			network_config.spawn_native().await
		},
		Provider::Docker => {
			info!("Using Docker provider");
			network_config.spawn_docker().await
		},
		Provider::K8s => {
			info!("Using Kubernetes provider");
			network_config.spawn_k8s().await
		},
	}
	.context("Failed to spawn network")?;

	info!("Network spawned successfully!");

	// Print node information
	for node in network.relaychain().nodes() {
		info!("Relay node '{}' running at {}", node.name(), node.ws_uri());
	}

	for para in network.parachains() {
		info!("Teyrchain ID {}", para.para_id());
		for collator in para.collators() {
			info!("  Collator '{}' running at {}", collator.name(), collator.ws_uri());
		}
	}

	info!("Press Ctrl+C to stop the network...");

	// Keep the network running until interrupted
	tokio::signal::ctrl_c().await?;

	info!("Shutting down network...");

	Ok(())
}
