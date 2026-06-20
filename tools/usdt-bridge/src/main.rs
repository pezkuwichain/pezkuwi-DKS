//! wUSDT Bridge - Custodial bridge for Wrapped USDT on Pezkuwi
//!
//! This bridge enables users to:
//! 1. Deposit USDT (Polkadot Asset Hub) -> Receive wUSDT on Pezkuwi Asset Hub
//! 2. Withdraw wUSDT (burn on Pezkuwi) -> Receive USDT on Polkadot Asset Hub
//!
//! Backing:
//! - 1:1 backed by real USDT on Polkadot Asset Hub
//!
//! Architecture:
//! - Custodial: A single keypair controls both sides
//! - Minimum deposit/withdraw: 10 USDT
//! - Fees: Configurable (default 0.1%)

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sp_core::{crypto::Ss58Codec, sr25519, Pair};
use std::path::PathBuf;
use subxt::{OnlineClient, SubstrateConfig};
use subxt::dynamic::{At, Value};
use subxt_signer::sr25519::Keypair;
use tracing::{info, warn, error};

// ============================================================================
// Configuration
// ============================================================================

/// Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Polkadot Asset Hub RPC endpoint
    pub polkadot_rpc: String,
    /// Pezkuwi Asset Hub RPC endpoint
    pub pezkuwi_rpc: String,
    /// USDT Asset ID on Polkadot Asset Hub (1984 is standard)
    pub polkadot_usdt_asset_id: u32,
    /// wUSDT Asset ID on Pezkuwi Asset Hub
    pub pezkuwi_wusdt_asset_id: u32,
    /// Minimum deposit amount (in USDT base units, 6 decimals)
    pub min_deposit: u128,
    /// Minimum withdraw amount
    pub min_withdraw: u128,
    /// Bridge fee percentage (e.g., 10 = 0.1%)
    pub fee_basis_points: u32,
    /// Bridge operator seed phrase path
    pub seed_path: PathBuf,
    /// Database path
    pub db_path: PathBuf,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            polkadot_rpc: "wss://polkadot-asset-hub-rpc.polkadot.io".to_string(),
            pezkuwi_rpc: "wss://asset-hub-rpc.pezkuwichain.io".to_string(),
            polkadot_usdt_asset_id: 1984,
            pezkuwi_wusdt_asset_id: 1000,
            min_deposit: 10_000_000,      // 10 USDT (6 decimals)
            min_withdraw: 10_000_000,
            fee_basis_points: 10,         // 0.1%
            seed_path: PathBuf::from("bridge_seed.json"),
            db_path: PathBuf::from("bridge_db.json"),
        }
    }
}

// ============================================================================
// CLI
// ============================================================================

#[derive(Parser)]
#[command(name = "usdt-bridge")]
#[command(about = "Custodial wUSDT Bridge: Polkadot <-> Pezkuwi")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path
    #[arg(short, long, default_value = "bridge_config.json")]
    config: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new bridge wallet keypair
    GenerateWallet {
        #[arg(short, long, default_value = "bridge_seed.json")]
        output: PathBuf,
    },

    /// Show bridge wallet addresses
    ShowAddresses {
        #[arg(short, long, default_value = "bridge_seed.json")]
        seed: PathBuf,
    },

    /// Start the deposit listener (Polkadot -> Pezkuwi)
    ListenDeposits,

    /// Process a single deposit manually
    ProcessDeposit {
        /// Polkadot tx hash
        #[arg(long)]
        tx_hash: String,
        /// Sender address (Polkadot)
        #[arg(long)]
        sender: String,
        /// Amount in USDT base units (6 decimals)
        #[arg(long)]
        amount: u128,
    },

    /// Process pending withdrawals (Pezkuwi -> Polkadot)
    ProcessWithdrawals,

    /// Mint wUSDT on Pezkuwi (manual)
    MintWusdt {
        /// Recipient address (Pezkuwi)
        #[arg(long)]
        to: String,
        /// Amount in USDT base units (6 decimals)
        #[arg(long)]
        amount: u128,
    },

    /// Transfer USDT on Polkadot (manual)
    TransferUsdt {
        /// Recipient address (Polkadot)
        #[arg(long)]
        to: String,
        /// Amount in USDT base units (6 decimals)
        #[arg(long)]
        amount: u128,
    },

    /// Show bridge status and balances
    Status,

    /// Initialize the database
    InitDb,

    /// Check balances on both chains
    Balances,
}

// ============================================================================
// Wallet Management
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct WalletSeed {
    mnemonic: String,
    polkadot_address: String,
    pezkuwi_address: String,
    public_key: String,
}

fn generate_wallet(output: &PathBuf) -> Result<()> {
    use sp_core::crypto::Ss58AddressFormat;

    let (pair, phrase, _) = sr25519::Pair::generate_with_phrase(None);

    let polkadot_address = pair.public().to_ss58check_with_version(Ss58AddressFormat::custom(0));
    let pezkuwi_address = pair.public().to_ss58check_with_version(Ss58AddressFormat::custom(42));
    let public_key = hex::encode(pair.public().0);

    let wallet = WalletSeed {
        mnemonic: phrase,
        polkadot_address: polkadot_address.clone(),
        pezkuwi_address: pezkuwi_address.clone(),
        public_key,
    };

    let json = serde_json::to_string_pretty(&wallet)?;
    std::fs::write(output, &json)?;

    println!("=== NEW BRIDGE WALLET GENERATED ===\n");
    println!("Polkadot Asset Hub: {}", polkadot_address);
    println!("Pezkuwi Asset Hub:  {}", pezkuwi_address);
    println!("\nSeed saved to: {}", output.display());
    println!("\nIMPORTANT: Back up the seed file securely!");

    Ok(())
}

fn show_addresses(seed_path: &PathBuf) -> Result<()> {
    let content = std::fs::read_to_string(seed_path).context("Failed to read seed file")?;
    let wallet: WalletSeed = serde_json::from_str(&content).context("Failed to parse seed")?;

    println!("=== BRIDGE WALLET ADDRESSES ===\n");
    println!("Polkadot Asset Hub: {}", wallet.polkadot_address);
    println!("Pezkuwi Asset Hub:  {}", wallet.pezkuwi_address);
    println!("Public Key:         0x{}", wallet.public_key);

    Ok(())
}

fn load_keypair(seed_path: &PathBuf) -> Result<Keypair> {
    let content = std::fs::read_to_string(seed_path).context("Failed to read seed file")?;
    let wallet: WalletSeed = serde_json::from_str(&content).context("Failed to parse seed")?;

    let mnemonic = bip39::Mnemonic::parse(&wallet.mnemonic)
        .map_err(|e| anyhow!("Invalid mnemonic: {:?}", e))?;

    let keypair = Keypair::from_phrase(&mnemonic, None)
        .map_err(|e| anyhow!("Failed to create keypair: {:?}", e))?;

    Ok(keypair)
}

fn load_wallet_addresses(seed_path: &PathBuf) -> Result<(String, String)> {
    let content = std::fs::read_to_string(seed_path).context("Failed to read seed file")?;
    let wallet: WalletSeed = serde_json::from_str(&content).context("Failed to parse seed")?;
    Ok((wallet.polkadot_address, wallet.pezkuwi_address))
}

// ============================================================================
// Database
// ============================================================================

#[derive(Debug, Default, Serialize, Deserialize)]
struct BridgeDatabase {
    deposits: Vec<DepositRecord>,
    withdrawals: Vec<WithdrawalRecord>,
    stats: BridgeStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DepositRecord {
    id: u64,
    polkadot_tx_hash: String,
    polkadot_block: u64,
    sender_address: String,
    amount: u128,
    fee: u128,
    net_amount: u128,
    pezkuwi_tx_hash: Option<String>,
    status: String,
    created_at: String,
    processed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WithdrawalRecord {
    id: u64,
    pezkuwi_tx_hash: String,
    pezkuwi_block: u64,
    sender_address: String,
    destination_address: String,
    amount: u128,
    fee: u128,
    net_amount: u128,
    polkadot_tx_hash: Option<String>,
    status: String,
    created_at: String,
    processed_at: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct BridgeStats {
    total_deposits: u64,
    total_withdrawals: u64,
    total_fees_collected: u128,
    last_polkadot_block: u64,
    last_pezkuwi_block: u64,
}

impl BridgeDatabase {
    fn load(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    fn save(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn add_deposit(&mut self, deposit: DepositRecord) {
        self.stats.total_deposits += 1;
        self.stats.total_fees_collected += deposit.fee;
        self.deposits.push(deposit);
    }

    fn next_deposit_id(&self) -> u64 {
        self.deposits.iter().map(|d| d.id).max().unwrap_or(0) + 1
    }
}

fn init_db(db_path: &PathBuf) -> Result<()> {
    if db_path.exists() {
        println!("Database already exists at: {}", db_path.display());
        return Ok(());
    }

    let db = BridgeDatabase::default();
    db.save(db_path)?;
    println!("Database initialized at: {}", db_path.display());
    Ok(())
}

// ============================================================================
// Chain Operations
// ============================================================================

async fn connect_to_chain(url: &str) -> Result<OnlineClient<SubstrateConfig>> {
    info!("Connecting to {}...", url);
    let client = OnlineClient::<SubstrateConfig>::from_url(url).await
        .context(format!("Failed to connect to {}", url))?;
    info!("Connected successfully");
    Ok(client)
}

/// Get asset balance using dynamic API
async fn get_asset_balance(
    client: &OnlineClient<SubstrateConfig>,
    asset_id: u32,
    account: &str,
) -> Result<u128> {
    // Decode account from SS58
    let account_bytes = sp_core::crypto::AccountId32::from_ss58check(account)
        .map_err(|e| anyhow!("Invalid account: {:?}", e))?;

    // Build storage query for Assets.Account
    let storage_query = subxt::dynamic::storage(
        "Assets",
        "Account",
        vec![
            Value::primitive(asset_id.into()),
            Value::from_bytes(<sp_core::crypto::AccountId32 as AsRef<[u8; 32]>>::as_ref(&account_bytes)),
        ],
    );

    let result = client.storage().at_latest().await?.fetch(&storage_query).await?;

    if let Some(value) = result {
        // Parse the balance from the storage value
        // AssetAccount { balance: u128, ... }
        if let Some(balance) = value.to_value()?.at("balance") {
            if let Some(b) = balance.as_u128() {
                return Ok(b);
            }
        }
    }

    Ok(0)
}

/// Get native balance
async fn get_native_balance(
    client: &OnlineClient<SubstrateConfig>,
    account: &str,
) -> Result<u128> {
    let account_bytes = sp_core::crypto::AccountId32::from_ss58check(account)
        .map_err(|e| anyhow!("Invalid account: {:?}", e))?;

    let storage_query = subxt::dynamic::storage(
        "System",
        "Account",
        vec![Value::from_bytes(<sp_core::crypto::AccountId32 as AsRef<[u8; 32]>>::as_ref(&account_bytes))],
    );

    let result = client.storage().at_latest().await?.fetch(&storage_query).await?;

    if let Some(value) = result {
        if let Some(data) = value.to_value()?.at("data") {
            if let Some(free) = data.at("free") {
                if let Some(b) = free.as_u128() {
                    return Ok(b);
                }
            }
        }
    }

    Ok(0)
}

/// Mint wUSDT on Pezkuwi Asset Hub
async fn mint_wusdt(
    client: &OnlineClient<SubstrateConfig>,
    keypair: &Keypair,
    asset_id: u32,
    to: &str,
    amount: u128,
) -> Result<String> {
    let to_bytes = sp_core::crypto::AccountId32::from_ss58check(to)
        .map_err(|e| anyhow!("Invalid recipient: {:?}", e))?;

    // Build Assets.mint call
    let call = subxt::dynamic::tx(
        "Assets",
        "mint",
        vec![
            Value::primitive(asset_id.into()),
            Value::unnamed_variant("Id", [Value::from_bytes(<sp_core::crypto::AccountId32 as AsRef<[u8; 32]>>::as_ref(&to_bytes))]),
            Value::primitive(amount.into()),
        ],
    );

    info!("Submitting mint transaction...");
    let tx_progress = client
        .tx()
        .sign_and_submit_then_watch_default(&call, keypair)
        .await?;

    let events = tx_progress.wait_for_finalized_success().await?;
    let tx_hash = format!("{:?}", events.extrinsic_hash());

    info!("Mint successful! TX: {}", tx_hash);
    Ok(tx_hash)
}

/// Transfer USDT on Polkadot Asset Hub
async fn transfer_usdt(
    client: &OnlineClient<SubstrateConfig>,
    keypair: &Keypair,
    asset_id: u32,
    to: &str,
    amount: u128,
) -> Result<String> {
    let to_bytes = sp_core::crypto::AccountId32::from_ss58check(to)
        .map_err(|e| anyhow!("Invalid recipient: {:?}", e))?;

    // Build Assets.transfer call
    let call = subxt::dynamic::tx(
        "Assets",
        "transfer",
        vec![
            Value::primitive(asset_id.into()),
            Value::unnamed_variant("Id", [Value::from_bytes(<sp_core::crypto::AccountId32 as AsRef<[u8; 32]>>::as_ref(&to_bytes))]),
            Value::primitive(amount.into()),
        ],
    );

    info!("Submitting transfer transaction...");
    let tx_progress = client
        .tx()
        .sign_and_submit_then_watch_default(&call, keypair)
        .await?;

    let events = tx_progress.wait_for_finalized_success().await?;
    let tx_hash = format!("{:?}", events.extrinsic_hash());

    info!("Transfer successful! TX: {}", tx_hash);
    Ok(tx_hash)
}

// ============================================================================
// Bridge Operations
// ============================================================================

async fn show_status(config: &BridgeConfig) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              wUSDT BRIDGE STATUS                             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Configuration:");
    println!("  Polkadot RPC:       {}", config.polkadot_rpc);
    println!("  Pezkuwi RPC:        {}", config.pezkuwi_rpc);
    println!("  Polkadot USDT ID:   {}", config.polkadot_usdt_asset_id);
    println!("  Pezkuwi wUSDT ID:   {}", config.pezkuwi_wusdt_asset_id);
    println!("  Min Deposit:        {} USDT", config.min_deposit as f64 / 1_000_000.0);
    println!("  Min Withdraw:       {} USDT", config.min_withdraw as f64 / 1_000_000.0);
    println!("  Fee:                {}%", config.fee_basis_points as f64 / 100.0);
    println!();

    // Load wallet
    if config.seed_path.exists() {
        let (polkadot_addr, pezkuwi_addr) = load_wallet_addresses(&config.seed_path)?;
        println!("Bridge Wallet:");
        println!("  Polkadot: {}", polkadot_addr);
        println!("  Pezkuwi:  {}", pezkuwi_addr);
    } else {
        println!("WARNING: No bridge wallet found. Run 'generate-wallet' first.");
    }

    // Load database stats
    if config.db_path.exists() {
        let db = BridgeDatabase::load(&config.db_path)?;
        println!("\nStatistics:");
        println!("  Total Deposits:     {}", db.stats.total_deposits);
        println!("  Total Withdrawals:  {}", db.stats.total_withdrawals);
        println!("  Fees Collected:     {} USDT", db.stats.total_fees_collected as f64 / 1_000_000.0);
    }

    Ok(())
}

async fn show_balances(config: &BridgeConfig) -> Result<()> {
    let (polkadot_addr, pezkuwi_addr) = load_wallet_addresses(&config.seed_path)?;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              BRIDGE WALLET BALANCES                          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Connect to Polkadot Asset Hub
    println!("Connecting to Polkadot Asset Hub...");
    match connect_to_chain(&config.polkadot_rpc).await {
        Ok(polkadot_client) => {
            let usdt_balance = get_asset_balance(
                &polkadot_client,
                config.polkadot_usdt_asset_id,
                &polkadot_addr,
            ).await.unwrap_or(0);

            let native_balance = get_native_balance(&polkadot_client, &polkadot_addr)
                .await.unwrap_or(0);

            println!("\nPolkadot Asset Hub ({}):", polkadot_addr);
            println!("  USDT:   {} USDT", usdt_balance as f64 / 1_000_000.0);
            println!("  Native: {} DOT", native_balance as f64 / 10_000_000_000.0);
        }
        Err(e) => {
            warn!("Could not connect to Polkadot: {}", e);
        }
    }

    // Connect to Pezkuwi Asset Hub
    println!("\nConnecting to Pezkuwi Asset Hub...");
    match connect_to_chain(&config.pezkuwi_rpc).await {
        Ok(pezkuwi_client) => {
            let wusdt_balance = get_asset_balance(
                &pezkuwi_client,
                config.pezkuwi_wusdt_asset_id,
                &pezkuwi_addr,
            ).await.unwrap_or(0);

            let native_balance = get_native_balance(&pezkuwi_client, &pezkuwi_addr)
                .await.unwrap_or(0);

            println!("\nPezkuwi Asset Hub ({}):", pezkuwi_addr);
            println!("  wUSDT:  {} USDT", wusdt_balance as f64 / 1_000_000.0);
            println!("  Native: {} HEZ", native_balance as f64 / 1_000_000_000_000.0);
        }
        Err(e) => {
            warn!("Could not connect to Pezkuwi: {}", e);
        }
    }

    Ok(())
}

fn calculate_fee(amount: u128, fee_basis_points: u32) -> u128 {
    amount * fee_basis_points as u128 / 10_000
}

async fn process_deposit(
    config: &BridgeConfig,
    tx_hash: &str,
    sender: &str,
    amount: u128,
) -> Result<()> {
    // Validate amount
    if amount < config.min_deposit {
        return Err(anyhow!(
            "Amount {} below minimum deposit {}",
            amount as f64 / 1_000_000.0,
            config.min_deposit as f64 / 1_000_000.0
        ));
    }

    // Calculate fee
    let fee = calculate_fee(amount, config.fee_basis_points);
    let net_amount = amount - fee;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              PROCESSING DEPOSIT                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Polkadot TX:    {}", tx_hash);
    println!("Sender:         {}", sender);
    println!("Amount:         {} USDT", amount as f64 / 1_000_000.0);
    println!("Fee ({}%):      {} USDT",
        config.fee_basis_points as f64 / 100.0,
        fee as f64 / 1_000_000.0
    );
    println!("Net Amount:     {} USDT", net_amount as f64 / 1_000_000.0);
    println!();

    // Load keypair
    let keypair = load_keypair(&config.seed_path)?;

    // Connect to Pezkuwi and mint wUSDT
    let pezkuwi_client = connect_to_chain(&config.pezkuwi_rpc).await?;

    // Convert sender address to Pezkuwi format (same public key, different SS58 prefix)
    // For custodial bridge, we need the user to provide their Pezkuwi address
    // For now, use sender address converted to Pezkuwi format
    let sender_account = sp_core::crypto::AccountId32::from_ss58check(sender)
        .map_err(|e| anyhow!("Invalid sender address: {:?}", e))?;
    let pezkuwi_recipient = sender_account.to_ss58check_with_version(
        sp_core::crypto::Ss58AddressFormat::custom(42)
    );

    println!("Minting {} wUSDT to {}...", net_amount as f64 / 1_000_000.0, pezkuwi_recipient);

    let pezkuwi_tx = mint_wusdt(
        &pezkuwi_client,
        &keypair,
        config.pezkuwi_wusdt_asset_id,
        &pezkuwi_recipient,
        net_amount,
    ).await?;

    // Save to database
    let mut db = BridgeDatabase::load(&config.db_path)?;
    let deposit = DepositRecord {
        id: db.next_deposit_id(),
        polkadot_tx_hash: tx_hash.to_string(),
        polkadot_block: 0, // Would need to fetch from chain
        sender_address: sender.to_string(),
        amount,
        fee,
        net_amount,
        pezkuwi_tx_hash: Some(pezkuwi_tx.clone()),
        status: "completed".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        processed_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    db.add_deposit(deposit);
    db.save(&config.db_path)?;

    println!("\n✅ DEPOSIT PROCESSED SUCCESSFULLY!");
    println!("   Pezkuwi TX: {}", pezkuwi_tx);

    Ok(())
}

async fn cmd_mint_wusdt(config: &BridgeConfig, to: &str, amount: u128) -> Result<()> {
    println!("Minting {} wUSDT to {}...", amount as f64 / 1_000_000.0, to);

    let keypair = load_keypair(&config.seed_path)?;
    let client = connect_to_chain(&config.pezkuwi_rpc).await?;

    let tx_hash = mint_wusdt(&client, &keypair, config.pezkuwi_wusdt_asset_id, to, amount).await?;

    println!("\n✅ MINT SUCCESSFUL!");
    println!("   TX Hash: {}", tx_hash);

    Ok(())
}

async fn cmd_transfer_usdt(config: &BridgeConfig, to: &str, amount: u128) -> Result<()> {
    println!("Transferring {} USDT to {}...", amount as f64 / 1_000_000.0, to);

    let keypair = load_keypair(&config.seed_path)?;
    let client = connect_to_chain(&config.polkadot_rpc).await?;

    let tx_hash = transfer_usdt(&client, &keypair, config.polkadot_usdt_asset_id, to, amount).await?;

    println!("\n✅ TRANSFER SUCCESSFUL!");
    println!("   TX Hash: {}", tx_hash);

    Ok(())
}

async fn listen_deposits(config: &BridgeConfig) -> Result<()> {
    let (polkadot_addr, _) = load_wallet_addresses(&config.seed_path)?;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              DEPOSIT LISTENER                                ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Bridge Address (Polkadot): {}", polkadot_addr);
    println!("USDT Asset ID: {}", config.polkadot_usdt_asset_id);
    println!("Min Deposit: {} USDT", config.min_deposit as f64 / 1_000_000.0);
    println!("\nListening for deposits...\n");

    let client = connect_to_chain(&config.polkadot_rpc).await?;
    let keypair = load_keypair(&config.seed_path)?;

    // Subscribe to new blocks
    let mut blocks = client.blocks().subscribe_finalized().await?;

    while let Some(block) = blocks.next().await {
        let block = block?;
        let block_number = block.number();

        // Get events for this block
        let events = block.events().await?;

        for event in events.iter() {
            let event = event?;

            // Check for Assets.Transferred event
            if event.pallet_name() == "Assets" && event.variant_name() == "Transferred" {
                // Parse event data
                if let Ok(fields) = event.field_values() {
                    // Fields: asset_id, from, to, amount
                    let asset_id = fields.at("asset_id")
                        .and_then(|v| v.as_u128())
                        .unwrap_or(0) as u32;

                    if asset_id != config.polkadot_usdt_asset_id {
                        continue;
                    }

                    let to_field = fields.at("to");
                    let from_field = fields.at("from");
                    let amount = fields.at("amount")
                        .and_then(|v| v.as_u128())
                        .unwrap_or(0);

                    // Check if transfer is to bridge address
                    if let Some(to_value) = to_field {
                        let to_str = format!("{:?}", to_value);
                        if to_str.contains(&polkadot_addr[..10]) {
                            let from_str = from_field.map(|f| format!("{:?}", f)).unwrap_or_default();

                            info!("📥 DEPOSIT DETECTED!");
                            info!("   Block: #{}", block_number);
                            info!("   From: {}", from_str);
                            info!("   Amount: {} USDT", amount as f64 / 1_000_000.0);

                            if amount >= config.min_deposit {
                                // Process deposit
                                let fee = calculate_fee(amount, config.fee_basis_points);
                                let net_amount = amount - fee;

                                // Get recipient Pezkuwi address
                                // In production, this should come from a memo or separate registration
                                // For now, convert sender's address

                                info!("   Processing... Fee: {} USDT, Net: {} USDT",
                                    fee as f64 / 1_000_000.0,
                                    net_amount as f64 / 1_000_000.0
                                );

                                // Connect to Pezkuwi and mint
                                match connect_to_chain(&config.pezkuwi_rpc).await {
                                    Ok(pezkuwi_client) => {
                                        // For demo, mint to bridge's own Pezkuwi address
                                        // In production, need user's Pezkuwi address
                                        let (_, pezkuwi_bridge) = load_wallet_addresses(&config.seed_path)?;

                                        match mint_wusdt(
                                            &pezkuwi_client,
                                            &keypair,
                                            config.pezkuwi_wusdt_asset_id,
                                            &pezkuwi_bridge,
                                            net_amount,
                                        ).await {
                                            Ok(tx) => {
                                                info!("   ✅ Minted wUSDT! TX: {}", tx);
                                            }
                                            Err(e) => {
                                                error!("   ❌ Mint failed: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("   ❌ Failed to connect to Pezkuwi: {}", e);
                                    }
                                }
                            } else {
                                warn!("   ⚠️ Amount below minimum, skipping");
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn process_withdrawals(config: &BridgeConfig) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              WITHDRAWAL PROCESSOR                            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("This would process pending withdrawals from the database.");
    println!("Withdrawals require burning wUSDT on Pezkuwi first.\n");

    // Load database
    let db = BridgeDatabase::load(&config.db_path)?;

    let pending: Vec<_> = db.withdrawals.iter()
        .filter(|w| w.status == "pending")
        .collect();

    if pending.is_empty() {
        println!("No pending withdrawals.");
    } else {
        println!("Pending withdrawals: {}", pending.len());
        for w in pending {
            println!("  #{}: {} USDT -> {}", w.id, w.amount as f64 / 1_000_000.0, w.destination_address);
        }
    }

    Ok(())
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("usdt_bridge=info".parse().unwrap())
                .add_directive("subxt=warn".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    // Load or create config
    let config: BridgeConfig = if cli.config.exists() {
        let content = std::fs::read_to_string(&cli.config)?;
        serde_json::from_str(&content)?
    } else {
        let default_config = BridgeConfig::default();
        let json = serde_json::to_string_pretty(&default_config)?;
        std::fs::write(&cli.config, &json)?;
        info!("Created default config at: {}", cli.config.display());
        default_config
    };

    match cli.command {
        Commands::GenerateWallet { output } => {
            generate_wallet(&output)?;
        }
        Commands::ShowAddresses { seed } => {
            show_addresses(&seed)?;
        }
        Commands::ListenDeposits => {
            listen_deposits(&config).await?;
        }
        Commands::ProcessDeposit { tx_hash, sender, amount } => {
            process_deposit(&config, &tx_hash, &sender, amount).await?;
        }
        Commands::ProcessWithdrawals => {
            process_withdrawals(&config).await?;
        }
        Commands::MintWusdt { to, amount } => {
            cmd_mint_wusdt(&config, &to, amount).await?;
        }
        Commands::TransferUsdt { to, amount } => {
            cmd_transfer_usdt(&config, &to, amount).await?;
        }
        Commands::Status => {
            show_status(&config).await?;
        }
        Commands::InitDb => {
            init_db(&config.db_path)?;
        }
        Commands::Balances => {
            show_balances(&config).await?;
        }
    }

    Ok(())
}
