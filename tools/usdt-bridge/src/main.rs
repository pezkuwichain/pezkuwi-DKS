//! wUSDT Bridge - Custodial bridge for Wrapped USDT on Pezkuwi
//!
//! This bridge enables users to:
//! 1. Deposit USDT (Polkadot Asset Hub) -> Receive wUSDT on Pezkuwi Asset Hub
//! 2. Withdraw wUSDT (burn on Pezkuwi) -> Receive USDT on Polkadot Asset Hub
//!
//! Backing:
//! - 1:1 backed by real USDT on Polkadot Asset Hub
//!
//! Architecture (updated 2026-07-13 - see the operator's custody notes for the full
//! migration record):
//! - Custody is a 3-of-5 multisig, NOT a single keypair. On Pezkuwi Asset Hub the multisig
//!   (5GvwxmCDp3PC33KHoeWSgj3S7ocE7nzk1jiCCZMPSDBFeNcj) now owns wUSDT (asset 1000) - it holds
//!   a seed pool of 1,000,000 wUSDT and is the sole issuer/admin/freezer. On Polkadot Asset Hub
//!   the same account (15sF76THfpefUaKomHZSpssayRbsp6Yt6ESgMrLjzJCmpe66) is the designated
//!   deposit address, though it currently holds no real USDT - nobody has deposited real value
//!   into this bridge yet.
//! - Because of this, `listen_deposits`/`listen_withdrawals` below can no longer sign and
//!   submit mint/transfer calls automatically - no key this process holds has authority over
//!   the multisig account. They now only DETECT on-chain events and record them as
//!   pending_multisig_approval; an actual release requires 3 of the 5 signatories (Serok,
//!   SerokiMeclise, Xezinedar, Berdevk, Noter - see the operator's custody notes) to approve via
//!   `Multisig.approveAsMulti`/`asMulti`, which is expected to happen through pwap-web's
//!   existing multisig UI (shared/lib/multisig.ts, shared/lib/usdt.ts, MultisigMembers.tsx),
//!   not this relayer.
//! - Minimum deposit/withdraw: 1 USDT (matches wallet-android's advertised minimum - this was a
//!   leftover scaffold default of 10 USDT that nobody had reconciled with the app's own 1 USDT
//!   minimum until a real user swap below 10 but above 1 USDT was silently skipped)
//! - Fees: Configurable (default 0.1%)

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sp_core::{crypto::Ss58Codec, sr25519, Pair};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;
use subxt::{OnlineClient, SubstrateConfig};
use subxt::backend::legacy::LegacyRpcMethods;
use subxt::backend::rpc::RpcClient;
use subxt::dynamic::{At, Value};
use subxt_signer::bip39::Mnemonic;
use subxt_signer::sr25519::Keypair as PolkadotKeypair;
// Pezkuwi-side signing MUST go through this fork, not vanilla subxt - vanilla subxt's default
// extrinsic params don't match Pezkuwi's signed-extension/hasher setup and produce "bad
// signature" errors on submission (see examples/migrate_wusdt_to_multisig.rs, which hit this
// first). Aliased to avoid clashing with the vanilla `Value`/`OnlineClient` already imported
// above and used throughout this file for read-only queries on both chains.
use pezkuwi_subxt::dynamic::Value as PValue;
use pezkuwi_subxt::{OnlineClient as PezkuwiSigningClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic as PezkuwiMnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair as PezkuwiKeypair;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{info, warn, error};

// The automation key is a deliberately limited, server-held delegate: it can only ever move
// funds the multisig has explicitly pre-approved via `Assets.approve_transfer`, up to whatever
// allowance remains, and only via `transfer_approved` (never mint/burn/set_team/transfer_ownership
// - those stay Owner/Admin-only, i.e. multisig-only, forever). This is categorically different
// from the 5 signatories' own seeds - see the standing rule in this session: never load a
// signatory's seed programmatically to bypass human multisig approval. This key exists
// specifically so it CAN be loaded programmatically; that is its whole purpose.
const AUTOMATION_KEY_ADDRESS: &str = "5GQu4PFUb1f3MTJ7i7c1CtLgDk3TVvpSW1VbQCRmfkMoC8cM";

/// A live `subscribe_finalized()` stream can silently stop yielding blocks - no error, no closed
/// stream, the process just stops seeing new blocks forever - while staying up and logging
/// nothing further ("Listening for deposits..." was the last line for 3+ hours while a real 1
/// USDT deposit sat undetected at block #18250967, ~62 minutes into that silence - see
/// the operator's custody notes 2026-07-15). Checkpoint + backfill only covers a gap across a
/// *restart*; this covers a stall with no restart at all. If no new finalized block arrives
/// within this window (Asset Hub blocks land every ~6-12s under normal conditions, so this is a
/// wide margin, not a tight one), treat the subscription as stalled and force a full reconnect -
/// the existing checkpoint-vs-head backfill then catches up whatever was missed during the stall.
const LIVE_SUBSCRIPTION_STALL_TIMEOUT: Duration = Duration::from_secs(120);

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
    /// Bridge operator seed phrase path (fallback only - prefer BRIDGE_SEED_MNEMONIC env var)
    pub seed_path: PathBuf,
    /// Database path
    pub db_path: PathBuf,
    /// Per-transaction ceiling the automation key may execute on its own (base units). Anything
    /// above this ALWAYS queues for manual 3-of-5 review, regardless of remaining daily budget -
    /// caps the blast radius of a single bad event (bug, reorg, compromised sender, compromised
    /// automation key) to one transaction's worth, no matter how much daily headroom exists.
    #[serde(default = "default_max_single_tx")]
    pub max_single_tx: u128,
    /// Rolling 24h ceiling on total value the automation key may auto-pay out, tracked
    /// independently per leg (deposit payouts in wUSDT vs withdrawal payouts in USDT draw on
    /// separate on-chain allowances on separate chains). Bounds worst-case exposure from a
    /// compromised automation key to one day's volume rather than an unbounded amount, while
    /// still letting many legitimate sub-cap swaps clear same-day without human involvement.
    #[serde(default = "default_auto_pay_daily_cap")]
    pub auto_pay_daily_cap: u128,
    /// Circuit-breaker window: if `circuit_breaker_fraction` of the daily cap is consumed within
    /// this many minutes, auto-pay halts entirely (falls back to manual review) until a human
    /// clears it via the `reset-circuit-breaker` command - a burst that fast is far more
    /// consistent with a compromised key or a bug than organic demand.
    #[serde(default = "default_circuit_breaker_window_minutes")]
    pub circuit_breaker_window_minutes: i64,
    #[serde(default = "default_circuit_breaker_fraction")]
    pub circuit_breaker_fraction: f64,
    /// The 3-of-5 multisig custody account, Polkadot Asset Hub SS58 encoding. Deposits are
    /// watched for transfers TO this address - it replaced the old single-key custody address.
    #[serde(default = "default_multisig_polkadot")]
    pub custody_multisig_polkadot: String,
    /// Same multisig account, Pezkuwi Asset Hub SS58 encoding (it owns wUSDT asset 1000).
    #[serde(default = "default_multisig_pezkuwi")]
    pub custody_multisig_pezkuwi: String,
    /// Telegram chat to page on a reserve shortfall or similar safety-critical signal - same
    /// chat PezbridgeBot already notifies (see src/bin/pezbridge_bot.rs), reused rather than
    /// standing up a second notification channel. 0 = disabled.
    #[serde(default)]
    pub serok_chat_id: i64,
    /// Same, for the ops group chat.
    #[serde(default)]
    pub ops_chat_id: i64,
    /// Below this, a SUCCESSFUL auto-pay stays silent on Telegram - at real user volume, a
    /// message per small routine swap would bury the notifications that actually need a human's
    /// attention (failures, amounts over max_single_tx, reserve problems, anything that fell to
    /// manual 3-of-5 review) under noise. Those all notify unconditionally regardless of amount -
    /// only the routine "everything worked, nothing to look at" case is gated by this.
    #[serde(default = "default_notify_threshold")]
    pub notify_threshold: u128,
}

fn default_max_single_tx() -> u128 {
    50_000_000_000 // 50,000 USDT (6 decimals) - small/mid-size swaps clear without waiting on 3-of-5
}

fn default_notify_threshold() -> u128 {
    500_000_000 // 500 USDT (6 decimals)
}

fn default_multisig_polkadot() -> String {
    "15sF76THfpefUaKomHZSpssayRbsp6Yt6ESgMrLjzJCmpe66".to_string()
}

fn default_multisig_pezkuwi() -> String {
    "5GvwxmCDp3PC33KHoeWSgj3S7ocE7nzk1jiCCZMPSDBFeNcj".to_string()
}

fn default_auto_pay_daily_cap() -> u128 {
    200_000_000_000 // 200,000 USDT/day - rolling 24h automation budget, per leg
}

fn default_circuit_breaker_window_minutes() -> i64 {
    30
}

fn default_circuit_breaker_fraction() -> f64 {
    0.5 // >=50% of the daily cap consumed within the window trips the breaker
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            polkadot_rpc: "wss://polkadot-asset-hub-rpc.polkadot.io".to_string(),
            pezkuwi_rpc: "wss://asset-hub-rpc.pezkuwichain.io".to_string(),
            polkadot_usdt_asset_id: 1984,
            pezkuwi_wusdt_asset_id: 1000,
            min_deposit: 1_000_000,      // 1 USDT (6 decimals) - matches wallet-android's advertised minimum
            min_withdraw: 1_000_000,
            fee_basis_points: 10,         // 0.1%
            seed_path: PathBuf::from("bridge_seed.json"),
            db_path: PathBuf::from("bridge_db.json"),
            max_single_tx: default_max_single_tx(),
            custody_multisig_polkadot: default_multisig_polkadot(),
            custody_multisig_pezkuwi: default_multisig_pezkuwi(),
            serok_chat_id: 0,
            ops_chat_id: 0,
            auto_pay_daily_cap: default_auto_pay_daily_cap(),
            circuit_breaker_window_minutes: default_circuit_breaker_window_minutes(),
            circuit_breaker_fraction: default_circuit_breaker_fraction(),
            notify_threshold: default_notify_threshold(),
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

    /// Start the withdrawal listener (Pezkuwi -> Polkadot)
    ListenWithdrawals,

    /// Start both listeners concurrently - the normal way to run this in production
    ListenAll,

    /// Report pending/failed withdrawals recorded so far (read-only; live processing
    /// happens automatically via ListenWithdrawals/ListenAll)
    ProcessWithdrawals,

    /// Show custody reserves vs circulating wUSDT supply (proof-of-reserves check)
    Reserves,

    /// Show bridge status and balances
    Status,

    /// Initialize the database
    InitDb,

    /// Check balances on both chains
    Balances,

    /// Show automation-key auto-pay status: rolling 24h volume per leg vs caps, and whether
    /// the circuit breaker is tripped
    AutomationStatus,

    /// Clear a tripped circuit breaker after human review - required before auto-pay resumes
    ResetCircuitBreaker,
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

/// Re-encodes an SS58 address under a different network's address-format prefix, keeping the
/// same underlying AccountId32 (public key). Safe here specifically because this wallet derives
/// the identical sr25519 key across every Substrate chain it supports (same seed, same path) -
/// so the sender's own address on chain A *is* their receiving address on chain B, just under a
/// different prefix. This must not be reused as a general "guess the recipient" trick outside
/// that specific guarantee.
fn convert_ss58_prefix(addr: &str, target_prefix: u16) -> Result<String> {
    let account = sp_core::crypto::AccountId32::from_ss58check(addr)
        .map_err(|e| anyhow!("Invalid address {}: {:?}", addr, e))?;
    Ok(account.to_ss58check_with_version(sp_core::crypto::Ss58AddressFormat::custom(target_prefix)))
}

/// Decodes an AccountId32 out of a dynamic event field, replacing the previous
/// Debug-string-contains-substring matching (never verified against a live node, and provably
/// wrong: it fed a `{:?}`-formatted value into convert_ss58_prefix, which can only parse real
/// SS58 text - that call could never have succeeded). AccountId32 fields decode as a composite
/// of 32 byte-sized primitives, sometimes wrapped in one extra single-item composite layer (the
/// same nesting observed in Tiki storage this session) - this peels exactly that.
fn account_id_from_dynamic(value: &subxt::ext::scale_value::Value<u32>) -> Option<[u8; 32]> {
    use subxt::ext::scale_value::{Composite, ValueDef};
    let mut current = value;
    loop {
        match &current.value {
            ValueDef::Composite(Composite::Unnamed(items)) => {
                if items.len() == 32 && items.iter().all(|v| v.as_u128().is_some()) {
                    let bytes: Vec<u8> =
                        items.iter().map(|v| v.as_u128().unwrap() as u8).collect();
                    return bytes.try_into().ok();
                } else if items.len() == 1 {
                    current = &items[0];
                    continue;
                }
                return None;
            }
            _ => return None,
        }
    }
}

fn account_bytes_to_ss58(bytes: [u8; 32], prefix: u16) -> String {
    sp_core::crypto::AccountId32::from(bytes)
        .to_ss58check_with_version(sp_core::crypto::Ss58AddressFormat::custom(prefix))
}

// ============================================================================
// Database
// ============================================================================

#[derive(Debug, Default, Serialize, Deserialize)]
struct BridgeDatabase {
    deposits: Vec<DepositRecord>,
    withdrawals: Vec<WithdrawalRecord>,
    stats: BridgeStats,
    /// Ledger of automation-key auto-payouts, used to compute rolling-window volume per leg.
    /// Pruned to the last 48h on every write - only ever needs a 24h lookback plus slack.
    #[serde(default)]
    auto_payouts: Vec<AutoPayoutRecord>,
    #[serde(default)]
    circuit_breaker_tripped: bool,
    #[serde(default)]
    circuit_breaker_reason: Option<String>,
    #[serde(default)]
    circuit_breaker_tripped_at: Option<String>,
}

/// One leg's identity for rolling-volume tracking - deposit payouts (wUSDT, Pezkuwi side) and
/// withdrawal payouts (USDT, Polkadot side) draw on separate on-chain allowances and must never
/// share a budget.
const LEG_DEPOSIT_WUSDT_PAYOUT: &str = "deposit_wusdt_payout";
const LEG_WITHDRAWAL_USDT_PAYOUT: &str = "withdrawal_usdt_payout";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AutoPayoutRecord {
    leg: String,
    amount: u128,
    at: String, // RFC3339
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
    /// Loads the DB, refusing to silently treat a corrupt/truncated file as empty - that would
    /// erase replay-protection history and risk re-processing everything already recorded. A
    /// genuinely missing file (first run) is the only case that gets a fresh, empty database.
    fn load(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            serde_json::from_str(&content)
                .with_context(|| format!("{} exists but is not valid JSON - refusing to reinitialize (would erase replay-protection history); restore from backup", path.display()))
        } else {
            Ok(Self::default())
        }
    }

    /// Write-temp-then-rename so a mid-write crash can never leave a truncated/corrupt file in
    /// place - `load()` would otherwise hard-fail on the next start.
    fn save(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, json)?;
        std::fs::rename(&tmp_path, path)?;
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

    fn add_withdrawal(&mut self, withdrawal: WithdrawalRecord) {
        self.stats.total_withdrawals += 1;
        self.stats.total_fees_collected += withdrawal.fee;
        self.withdrawals.push(withdrawal);
    }

    fn next_withdrawal_id(&self) -> u64 {
        self.withdrawals.iter().map(|w| w.id).max().unwrap_or(0) + 1
    }

    /// Idempotency guard: every deposit/withdrawal must be checked against processed history
    /// BEFORE minting/releasing, not just recorded after the fact - a relayer restart or a
    /// re-scanned block range must never be able to mint/release twice for the same source tx.
    fn is_deposit_processed(&self, polkadot_tx_hash: &str) -> bool {
        self.deposits.iter().any(|d| d.polkadot_tx_hash == polkadot_tx_hash)
    }

    fn is_withdrawal_processed(&self, pezkuwi_tx_hash: &str) -> bool {
        self.withdrawals.iter().any(|w| w.pezkuwi_tx_hash == pezkuwi_tx_hash)
    }

    /// Sum of auto-payouts on `leg` within the last `window_minutes` minutes, relative to `now`.
    /// Used for both the 24h daily-cap check (window_minutes = 1440) and the fast-burst
    /// circuit-breaker check (window_minutes = config.circuit_breaker_window_minutes).
    fn recent_auto_payout_volume(&self, leg: &str, now: DateTime<Utc>, window_minutes: i64) -> u128 {
        self.auto_payouts
            .iter()
            .filter(|p| p.leg == leg)
            .filter_map(|p| DateTime::parse_from_rfc3339(&p.at).ok().map(|t| (t.with_timezone(&Utc), p.amount)))
            .filter(|(t, _)| now.signed_duration_since(*t) < chrono::Duration::minutes(window_minutes))
            .map(|(_, a)| a)
            .sum()
    }

    /// Records a completed auto-payout and prunes entries older than 48h (only a 24h lookback is
    /// ever needed - the extra slack just avoids pruning right at the boundary of a live check).
    fn record_auto_payout(&mut self, leg: &str, amount: u128, now: DateTime<Utc>) {
        self.auto_payouts.push(AutoPayoutRecord { leg: leg.to_string(), amount, at: now.to_rfc3339() });
        let cutoff = now - chrono::Duration::hours(48);
        self.auto_payouts.retain(|p| {
            DateTime::parse_from_rfc3339(&p.at)
                .map(|t| t.with_timezone(&Utc) > cutoff)
                .unwrap_or(true) // keep anything unparseable rather than silently drop history
        });
    }

    fn trip_circuit_breaker(&mut self, reason: String, now: DateTime<Utc>) {
        self.circuit_breaker_tripped = true;
        self.circuit_breaker_reason = Some(reason);
        self.circuit_breaker_tripped_at = Some(now.to_rfc3339());
    }
}

/// Guards every load-mutate-save critical section on `BridgeDatabase`. `listen_deposits` and
/// `listen_withdrawals` run as concurrent tasks under `listen_all` and share one JSON file with
/// no other locking - without this, two events landing close together could race (one task's
/// load-mutate-save clobbering the other's freshly-written record). A single process-wide lock
/// is sufficient here (this binary is the only writer to this file).
static DB_LOCK: LazyLock<AsyncMutex<()>> = LazyLock::new(|| AsyncMutex::new(()));

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

/// Retries with exponential backoff (2s, 4s, 8s, 16s, 32s; capped, 6 attempts total) rather than
/// failing on the first transient error. This matters because every caller here either bubbles a
/// failure straight up through a `?` to the listener's outer reconnect loop (see listen_deposits/
/// listen_withdrawals) or all the way out of main() to a systemd restart - a single blip (a rate
/// limit, a DNS hiccup) would otherwise tear down and fully re-initialize both listeners instead
/// of just waiting the blip out. Observed live 2026-07-16: the public Polkadot Asset Hub RPC
/// briefly 429'd, which without this retry crashed the whole process and then kept re-crashing
/// every RestartSec, hammering the same rate-limited endpoint harder instead of backing off.
async fn connect_to_chain(url: &str) -> Result<OnlineClient<SubstrateConfig>> {
    const MAX_ATTEMPTS: u32 = 6;
    let mut delay = Duration::from_secs(2);

    for attempt in 1..=MAX_ATTEMPTS {
        info!("Connecting to {}... (attempt {attempt}/{MAX_ATTEMPTS})", url);

        match OnlineClient::<SubstrateConfig>::from_url(url).await {
            Ok(client) => {
                info!("Connected successfully");
                return Ok(client);
            }
            Err(e) if attempt < MAX_ATTEMPTS => {
                warn!("Connect attempt {attempt}/{MAX_ATTEMPTS} to {url} failed: {e} - retrying in {}s", delay.as_secs());
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(60));
            }
            Err(e) => {
                return Err(e).context(format!("Failed to connect to {url} after {MAX_ATTEMPTS} attempts"));
            }
        }
    }

    unreachable!("loop always returns via Ok or the final Err arm")
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

// ============================================================================
// Telegram alerting (reuses PezbridgeBot's exact send pattern/chats - see
// src/bin/pezbridge_bot.rs - rather than standing up a second notification channel)
// ============================================================================

async fn tg_send_message(http: &reqwest::Client, token: &str, chat_id: i64, text: &str) -> Result<()> {
    if chat_id == 0 {
        return Ok(());
    }
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let body = serde_json::json!({ "chat_id": chat_id, "text": text });
    let resp = http.post(&url).json(&body).send().await?;
    let json: serde_json::Value = resp.json().await?;
    if json["ok"].as_bool() != Some(true) {
        return Err(anyhow!("sendMessage failed: {json}"));
    }
    Ok(())
}

/// Pages both configured chats for a safety-critical signal (reserve shortfall, stuck pending
/// row). Errors are logged, not propagated - a Telegram outage must never block/crash the
/// listener itself, it should just mean the page didn't go out this one time.
async fn notify_telegram(config: &BridgeConfig, msg: &str) {
    error!("[ALERT] {msg}");
    let Ok(token) = std::env::var("PEZBRIDGE_TELEGRAM_TOKEN") else {
        warn!("PEZBRIDGE_TELEGRAM_TOKEN not set - cannot page anyone for: {msg}");
        return;
    };
    let http = reqwest::Client::new();
    if let Err(e) = tg_send_message(&http, &token, config.serok_chat_id, msg).await {
        error!("failed to notify Serok: {e:#}");
    }
    if let Err(e) = tg_send_message(&http, &token, config.ops_chat_id, msg).await {
        error!("failed to notify ops chat: {e:#}");
    }
}

// ============================================================================
// Automation key: bounded auto-pay via Assets.transfer_approved
//
// The automation key holds ONLY a delegated `Assets.approve_transfer` allowance from the
// multisig - it can never mint, burn, set_team, or transfer_ownership. Its worst case (private
// key stolen) is bounded by whatever allowance remains on-chain at the time, which in turn is
// bounded by `max_single_tx` per transaction and kept small by design (renewed in modest
// top-ups, requiring fresh 3-of-5 approval each time - see pezbridge_bot.rs). This module adds
// two more independent layers on top of that on-chain bound: a rolling 24h volume cap per leg,
// and a burst-rate circuit breaker that halts all auto-pay if that cap is consumed too fast.
// ============================================================================

/// Both chain-specific signing keypairs for the SAME automation key (same seed, same sr25519
/// key - just wrapped in the two different subxt fork types each chain's extrinsic params need).
/// `None` fields mean the corresponding on-chain client failed to connect at startup; auto-pay
/// on that leg is then simply skipped (falls back to today's fully-manual pending-record path).
struct AutomationContext {
    pezkuwi_signing_client: Option<PezkuwiSigningClient<PezkuwiConfig>>,
    pezkuwi_keypair: Option<PezkuwiKeypair>,
    polkadot_signing_client: Option<OnlineClient<SubstrateConfig>>,
    polkadot_keypair: Option<PolkadotKeypair>,
}

impl AutomationContext {
    fn disabled() -> Self {
        Self { pezkuwi_signing_client: None, pezkuwi_keypair: None, polkadot_signing_client: None, polkadot_keypair: None }
    }
}

/// Loads the automation key's mnemonic from `AUTOMATION_KEY_MNEMONIC` and connects signing
/// clients on both chains. If the env var isn't set, returns a fully-disabled context - the
/// system behaves exactly as it did before this feature existed (100% manual multisig review).
/// This is deliberate: auto-pay must never be "on by default" just because the binary was
/// rebuilt: it activates only once an operator explicitly provisions the credential.
async fn init_automation_context(config: &BridgeConfig) -> AutomationContext {
    let Ok(mnemonic_str) = std::env::var("AUTOMATION_KEY_MNEMONIC") else {
        warn!("AUTOMATION_KEY_MNEMONIC not set - auto-pay disabled, all deposits/withdrawals will queue for manual 3-of-5 review (this is safe, just fully manual)");
        return AutomationContext::disabled();
    };

    let pezkuwi_keypair = (|| -> Result<PezkuwiKeypair> {
        let m = PezkuwiMnemonic::parse(&mnemonic_str).context("parsing automation mnemonic (pezkuwi)")?;
        let kp = PezkuwiKeypair::from_phrase(&m, None).context("deriving pezkuwi keypair")?;
        let addr = kp.public_key().to_account_id().to_string();
        if addr != AUTOMATION_KEY_ADDRESS {
            return Err(anyhow!("AUTOMATION_KEY_MNEMONIC derives {addr}, expected {AUTOMATION_KEY_ADDRESS} - refusing to use a mismatched key"));
        }
        Ok(kp)
    })();

    let polkadot_keypair = (|| -> Result<PolkadotKeypair> {
        let m = Mnemonic::parse(&mnemonic_str).context("parsing automation mnemonic (polkadot)")?;
        PolkadotKeypair::from_phrase(&m, None).context("deriving polkadot keypair")
    })();

    let (pezkuwi_keypair, polkadot_keypair) = match (pezkuwi_keypair, polkadot_keypair) {
        (Ok(p), Ok(d)) => (Some(p), Some(d)),
        (Err(e), _) | (_, Err(e)) => {
            error!("Failed to derive automation keypair(s), auto-pay disabled: {e:#}");
            return AutomationContext::disabled();
        }
    };

    let pezkuwi_signing_client = match PezkuwiSigningClient::<PezkuwiConfig>::from_insecure_url(&config.pezkuwi_rpc).await {
        Ok(c) => Some(c),
        Err(e) => { error!("Auto-pay (deposit/wUSDT leg) disabled - failed to connect signing client: {e:#}"); None }
    };
    let polkadot_signing_client = match OnlineClient::<SubstrateConfig>::from_url(&config.polkadot_rpc).await {
        Ok(c) => Some(c),
        Err(e) => { error!("Auto-pay (withdrawal/USDT leg) disabled - failed to connect signing client: {e:#}"); None }
    };

    info!("Automation key loaded ({AUTOMATION_KEY_ADDRESS}) - auto-pay ENABLED where signing clients connected");
    AutomationContext { pezkuwi_signing_client, pezkuwi_keypair, polkadot_signing_client, polkadot_keypair }
}

fn multi_address_id_pezkuwi(bytes: [u8; 32]) -> PValue {
    PValue::unnamed_variant("Id", [PValue::from_bytes(bytes)])
}

fn multi_address_id_polkadot(bytes: [u8; 32]) -> Value {
    Value::unnamed_variant("Id", [Value::from_bytes(bytes)])
}

/// Submits `Assets.transfer_approved(asset_id, owner, destination, amount)` on Pezkuwi Asset Hub,
/// signed by the automation key, and waits for on-chain dispatch success. Used for the
/// USDT->wUSDT deposit leg (paying wUSDT out of the multisig's seed pool).
async fn execute_wusdt_auto_payout(
    client: &PezkuwiSigningClient<PezkuwiConfig>,
    keypair: &PezkuwiKeypair,
    asset_id: u32,
    owner_bytes: [u8; 32],
    destination_bytes: [u8; 32],
    amount: u128,
) -> Result<String> {
    let call = pezkuwi_subxt::dynamic::tx(
        "Assets",
        "transfer_approved",
        vec![
            PValue::primitive(asset_id.into()),
            multi_address_id_pezkuwi(owner_bytes),
            multi_address_id_pezkuwi(destination_bytes),
            PValue::u128(amount),
        ],
    );
    let mut progress = client.tx().sign_and_submit_then_watch_default(&call, keypair).await
        .context("submitting transfer_approved (pezkuwi)")?;
    let hash = format!("0x{}", hex::encode(progress.extrinsic_hash().as_ref()));
    use pezkuwi_subxt::tx::TxStatus;
    loop {
        match progress.next().await {
            Some(Ok(TxStatus::InBestBlock(details))) => {
                details.wait_for_success().await.map_err(|e| anyhow!("transfer_approved dispatch failed: {e}"))?;
                return Ok(hash);
            }
            Some(Ok(TxStatus::Error { message })) => return Err(anyhow!("tx error: {message}")),
            Some(Ok(TxStatus::Invalid { message })) => return Err(anyhow!("tx invalid: {message}")),
            Some(Ok(TxStatus::Dropped { message })) => return Err(anyhow!("tx dropped: {message}")),
            Some(Err(e)) => return Err(anyhow!("stream error: {e}")),
            None => return Err(anyhow!("stream ended without a result")),
            _ => continue,
        }
    }
}

/// Same as `execute_wusdt_auto_payout` but on Polkadot Asset Hub via vanilla subxt (correct here
/// - the bad-signature gotcha only applies to Pezkuwi's custom chain). Used for the wUSDT->USDT
/// withdrawal leg (paying real USDT out of the multisig's Polkadot-side reserve).
async fn execute_usdt_auto_payout(
    client: &OnlineClient<SubstrateConfig>,
    keypair: &PolkadotKeypair,
    asset_id: u32,
    owner_bytes: [u8; 32],
    destination_bytes: [u8; 32],
    amount: u128,
) -> Result<String> {
    let call = subxt::dynamic::tx(
        "Assets",
        "transfer_approved",
        vec![
            Value::primitive(asset_id.into()),
            multi_address_id_polkadot(owner_bytes),
            multi_address_id_polkadot(destination_bytes),
            Value::u128(amount),
        ],
    );
    let mut progress = client.tx().sign_and_submit_then_watch_default(&call, keypair).await
        .context("submitting transfer_approved (polkadot)")?;
    let hash = format!("0x{}", hex::encode(progress.extrinsic_hash().as_ref()));
    use subxt::tx::TxStatus;
    loop {
        match progress.next().await {
            Some(Ok(TxStatus::InBestBlock(details))) => {
                details.wait_for_success().await.map_err(|e| anyhow!("transfer_approved dispatch failed: {e}"))?;
                return Ok(hash);
            }
            Some(Ok(TxStatus::Error { message })) => return Err(anyhow!("tx error: {message}")),
            Some(Ok(TxStatus::Invalid { message })) => return Err(anyhow!("tx invalid: {message}")),
            Some(Ok(TxStatus::Dropped { message })) => return Err(anyhow!("tx dropped: {message}")),
            Some(Err(e)) => return Err(anyhow!("stream error: {e}")),
            None => return Err(anyhow!("stream ended without a result")),
            _ => continue,
        }
    }
}

/// Shared eligibility gate for both legs: circuit breaker must be clear, amount within per-tx
/// cap (redundant with the caller's existing max_single_tx check, checked again here so this
/// function is safe to call from anywhere), and (existing + amount) must stay within the
/// rolling 24h cap for this leg. Returns the disqualifying reason as Err(reason) for logging/
/// alerting, or Ok(()) if auto-pay should proceed.
fn check_auto_pay_eligible(db: &BridgeDatabase, config: &BridgeConfig, leg: &str, amount: u128, now: DateTime<Utc>) -> std::result::Result<(), String> {
    if db.circuit_breaker_tripped {
        return Err(format!("circuit breaker is tripped ({}) - auto-pay halted until a human runs reset-circuit-breaker", db.circuit_breaker_reason.as_deref().unwrap_or("no reason recorded")));
    }
    if amount > config.max_single_tx {
        return Err(format!("amount {} exceeds max_single_tx {}", amount, config.max_single_tx));
    }
    let existing = db.recent_auto_payout_volume(leg, now, 24 * 60);
    if existing + amount > config.auto_pay_daily_cap {
        return Err(format!(
            "would exceed daily auto-pay cap for {leg}: {} already used + {} requested > {} cap",
            existing, amount, config.auto_pay_daily_cap
        ));
    }
    Ok(())
}

/// After a successful auto-payout, checks whether the burst-rate circuit breaker should trip -
/// i.e. whether `circuit_breaker_fraction` of the daily cap was consumed within
/// `circuit_breaker_window_minutes`. Call this AFTER `record_auto_payout` so the just-completed
/// payout is included in the window sum.
fn maybe_trip_circuit_breaker(db: &mut BridgeDatabase, config: &BridgeConfig, leg: &str, now: DateTime<Utc>) -> Option<String> {
    let recent = db.recent_auto_payout_volume(leg, now, config.circuit_breaker_window_minutes);
    let threshold = (config.auto_pay_daily_cap as f64 * config.circuit_breaker_fraction) as u128;
    if recent >= threshold {
        let reason = format!(
            "{leg}: {} auto-paid within {} min (>= {:.0}% of daily cap {}) - burst rate far exceeds normal organic demand",
            recent, config.circuit_breaker_window_minutes, config.circuit_breaker_fraction * 100.0, config.auto_pay_daily_cap
        );
        db.trip_circuit_breaker(reason.clone(), now);
        Some(reason)
    } else {
        None
    }
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

    println!("Custody (3-of-5 multisig):");
    println!("  Polkadot: {}", config.custody_multisig_polkadot);
    println!("  Pezkuwi:  {}", config.custody_multisig_pezkuwi);

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
    let polkadot_addr = config.custody_multisig_polkadot.clone();
    let pezkuwi_addr = config.custody_multisig_pezkuwi.clone();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              CUSTODY (3-OF-5 MULTISIG) BALANCES              ║");
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

/// Get an asset's total circulating supply (Assets.Asset(asset_id).supply)
async fn get_asset_total_supply(
    client: &OnlineClient<SubstrateConfig>,
    asset_id: u32,
) -> Result<u128> {
    let storage_query = subxt::dynamic::storage(
        "Assets",
        "Asset",
        vec![Value::primitive(asset_id.into())],
    );

    let result = client.storage().at_latest().await?.fetch(&storage_query).await?;

    if let Some(value) = result {
        if let Some(supply) = value.to_value()?.at("supply") {
            if let Some(s) = supply.as_u128() {
                return Ok(s);
            }
        }
    }

    Ok(0)
}

/// Proof-of-reserves: circulating wUSDT supply on Pezkuwi should never exceed real USDT
/// actually held in bridge custody on Polkadot. This is the exact invariant that, if broken,
/// produces the "waiting for liquidity" situation - surfacing it explicitly here instead of
/// leaving it as an opaque wallet-side banner.
async fn show_reserves(config: &BridgeConfig) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              PROOF OF RESERVES                                ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let polkadot_client = connect_to_chain(&config.polkadot_rpc).await?;
    let pezkuwi_client = connect_to_chain(&config.pezkuwi_rpc).await?;

    let real_reserve = get_asset_balance(&polkadot_client, config.polkadot_usdt_asset_id, &config.custody_multisig_polkadot).await?;
    let circulating_supply = get_asset_total_supply(&pezkuwi_client, config.pezkuwi_wusdt_asset_id).await?;

    println!("Real USDT locked (Polkadot custody):  {} USDT", real_reserve as f64 / 1_000_000.0);
    println!("Circulating wUSDT (Pezkuwi supply):    {} USDT", circulating_supply as f64 / 1_000_000.0);

    if circulating_supply > real_reserve {
        let shortfall = circulating_supply - real_reserve;
        println!(
            "\n🚨 UNDER-COLLATERALIZED by {} USDT - withdrawals beyond available reserve will \
             (correctly) be halted by listen_withdrawals' circuit breaker until this is resolved.",
            shortfall as f64 / 1_000_000.0
        );
    } else {
        println!(
            "\n✅ Fully backed, {} USDT of headroom.",
            (real_reserve - circulating_supply) as f64 / 1_000_000.0
        );
    }

    Ok(())
}

fn calculate_fee(amount: u128, fee_basis_points: u32) -> u128 {
    amount * fee_basis_points as u128 / 10_000
}

/// Processes exactly one block's deposit-relevant events. Called identically whether the block
/// arrived via the live finalized-block subscription or via backfill - this is the ONE place the
/// detection/auto-pay logic lives, so a gap in live coverage can never silently produce
/// different behavior than a backfilled scan of the same block would.
async fn process_deposit_block(
    config: &BridgeConfig,
    automation: &AutomationContext,
    block: &subxt::blocks::Block<SubstrateConfig, OnlineClient<SubstrateConfig>>,
) -> Result<()> {
    let polkadot_addr = &config.custody_multisig_polkadot;
    let block_number = block.number();
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

                    // Check if transfer is to bridge address - real byte comparison, not string
                    // matching (see account_id_from_dynamic's doc comment for why the previous
                    // Debug-string approach was not just fragile but provably broken).
                    let target_bytes = sp_core::crypto::AccountId32::from_ss58check(polkadot_addr)
                        .map(|a| Into::<[u8; 32]>::into(a))
                        .ok();
                    let to_bytes = to_field.and_then(account_id_from_dynamic);

                    if to_bytes.is_some() && to_bytes == target_bytes {
                        {
                            let from_bytes_opt = from_field.and_then(account_id_from_dynamic);
                            let from_str = from_bytes_opt.map(|b| account_bytes_to_ss58(b, 0)).unwrap_or_default();
                            let tx_hash = format!("block-{}-{}", block_number, from_str);

                            info!("📥 DEPOSIT DETECTED!");
                            info!("   Block: #{}", block_number);
                            info!("   From: {}", from_str);
                            info!("   Amount: {} USDT", amount as f64 / 1_000_000.0);

                            let _guard = DB_LOCK.lock().await;
                            let mut db = BridgeDatabase::load(&config.db_path)?;

                            if db.is_deposit_processed(&tx_hash) {
                                warn!("   ⚠️ Already processed ({}), skipping - refusing to mint twice", tx_hash);
                            } else if amount > config.max_single_tx {
                                warn!(
                                    "   ⚠️ Amount {} exceeds max_single_tx cap {} - recording as pending_multisig_approval_amount_exceeds_cap, needs manual multi-party review",
                                    amount as f64 / 1_000_000.0,
                                    config.max_single_tx as f64 / 1_000_000.0
                                );
                                notify_telegram(config, &format!(
                                    "📥 Deposit {} USDT from {} exceeds max_single_tx ({} USDT) - recorded, needs manual multi-party review.",
                                    amount as f64 / 1_000_000.0, from_str, config.max_single_tx as f64 / 1_000_000.0
                                )).await;
                                let fee = calculate_fee(amount, config.fee_basis_points);
                                let net_amount = amount - fee;
                                db.add_deposit(DepositRecord {
                                    id: db.next_deposit_id(),
                                    polkadot_tx_hash: tx_hash.clone(),
                                    polkadot_block: block_number as u64,
                                    sender_address: from_str.clone(),
                                    amount,
                                    fee,
                                    net_amount,
                                    pezkuwi_tx_hash: None,
                                    status: "pending_multisig_approval_amount_exceeds_cap".to_string(),
                                    created_at: Utc::now().to_rfc3339(),
                                    processed_at: None,
                                });
                                db.save(&config.db_path)?;
                            } else if amount >= config.min_deposit {
                                let fee = calculate_fee(amount, config.fee_basis_points);
                                let net_amount = amount - fee;

                                // Same AccountId32, re-encoded under Pezkuwi's SS58 prefix -
                                // see convert_ss58_prefix's doc comment for why this is
                                // correct rather than a placeholder.
                                match convert_ss58_prefix(&from_str, 42) {
                                    Ok(pezkuwi_recipient) => {
                                        let now = Utc::now();
                                        let auto_pay_attempt = match (from_bytes_opt, &automation.pezkuwi_signing_client, &automation.pezkuwi_keypair) {
                                            (Some(dest_bytes), Some(client), Some(keypair)) => {
                                                match check_auto_pay_eligible(&db, config, LEG_DEPOSIT_WUSDT_PAYOUT, net_amount, now) {
                                                    Ok(()) => {
                                                        let owner_bytes = sp_core::crypto::AccountId32::from_ss58check(&config.custody_multisig_pezkuwi)
                                                            .map(Into::<[u8; 32]>::into)
                                                            .map_err(|e| format!("bad custody_multisig_pezkuwi config: {e:?}"));
                                                        match owner_bytes {
                                                            Ok(owner_bytes) => {
                                                                let first = execute_wusdt_auto_payout(client, keypair, config.pezkuwi_wusdt_asset_id, owner_bytes, dest_bytes, net_amount).await;
                                                                // A cached signing client's WS connection can die silently between
                                                                // uses (same failure class as the listener's live subscription -
                                                                // see LIVE_SUBSCRIPTION_STALL_TIMEOUT's doc) with no error until the
                                                                // next real use - retry once against a genuinely fresh connection
                                                                // before falling back to manual review, rather than requiring a
                                                                // full process restart to recover auto-pay.
                                                                let result = if first.is_err() {
                                                                    warn!("   ⚠️ Auto-pay attempt failed ({:?}) - retrying once with a fresh connection", first.as_ref().err());
                                                                    match PezkuwiSigningClient::<PezkuwiConfig>::from_insecure_url(&config.pezkuwi_rpc).await {
                                                                        Ok(fresh_client) => execute_wusdt_auto_payout(&fresh_client, keypair, config.pezkuwi_wusdt_asset_id, owner_bytes, dest_bytes, net_amount).await,
                                                                        Err(_) => first,
                                                                    }
                                                                } else {
                                                                    first
                                                                };
                                                                Some(result)
                                                            }
                                                            Err(reason) => { warn!("   ⚠️ Auto-pay skipped: {reason}"); None }
                                                        }
                                                    }
                                                    Err(reason) => { info!("   ℹ️ Not auto-paying (falls back to manual): {reason}"); None }
                                                }
                                            }
                                            _ => None, // automation not configured/connected - always-manual fallback
                                        };

                                        match auto_pay_attempt {
                                            Some(Ok(pezkuwi_tx_hash)) => {
                                                info!("   ✅ AUTO-PAID: {} wUSDT -> {} (tx {})", net_amount as f64 / 1_000_000.0, pezkuwi_recipient, pezkuwi_tx_hash);
                                                db.record_auto_payout(LEG_DEPOSIT_WUSDT_PAYOUT, net_amount, now);
                                                let breaker_tripped = maybe_trip_circuit_breaker(&mut db, config, LEG_DEPOSIT_WUSDT_PAYOUT, now);
                                                db.add_deposit(DepositRecord {
                                                    id: db.next_deposit_id(),
                                                    polkadot_tx_hash: tx_hash.clone(),
                                                    polkadot_block: block_number as u64,
                                                    sender_address: from_str.clone(),
                                                    amount, fee, net_amount,
                                                    pezkuwi_tx_hash: Some(pezkuwi_tx_hash.clone()),
                                                    status: "auto_completed".to_string(),
                                                    created_at: now.to_rfc3339(),
                                                    processed_at: Some(now.to_rfc3339()),
                                                });
                                                db.save(&config.db_path)?;
                                                if let Some(reason) = breaker_tripped {
                                                    notify_telegram(config, &format!("🚨 CIRCUIT BREAKER TRIPPED (deposit/wUSDT leg): {reason}. All further deposits now queue for manual 3-of-5 review until `reset-circuit-breaker` is run after human review.")).await;
                                                } else if amount >= config.notify_threshold {
                                                    notify_telegram(config, &format!(
                                                        "✅ Auto-paid deposit: {} USDT from {} -> {} wUSDT (net of fee) to {}. tx {}",
                                                        amount as f64 / 1_000_000.0, from_str, net_amount as f64 / 1_000_000.0, pezkuwi_recipient, pezkuwi_tx_hash
                                                    )).await;
                                                }
                                            }
                                            Some(Err(e)) => {
                                                error!("   ❌ Auto-pay execution FAILED, falling back to manual review: {e:#}");
                                                notify_telegram(config, &format!(
                                                    "⚠️ Auto-pay attempt failed for deposit {} USDT from {} - falling back to manual 3-of-5 review. Error: {e:#}",
                                                    amount as f64 / 1_000_000.0, from_str
                                                )).await;
                                                db.add_deposit(DepositRecord {
                                                    id: db.next_deposit_id(),
                                                    polkadot_tx_hash: tx_hash.clone(),
                                                    polkadot_block: block_number as u64,
                                                    sender_address: from_str.clone(),
                                                    amount, fee, net_amount,
                                                    pezkuwi_tx_hash: None,
                                                    status: "pending_multisig_approval".to_string(),
                                                    created_at: now.to_rfc3339(),
                                                    processed_at: None,
                                                });
                                                db.save(&config.db_path)?;
                                            }
                                            None => {
                                                info!("   ⏳ AWAITING MULTISIG APPROVAL - not auto-minting.");
                                                info!(
                                                    "      3 of 5 signatories must approve: Assets.transfer({}, {}, {}) from the multisig's seed pool",
                                                    config.pezkuwi_wusdt_asset_id, pezkuwi_recipient, net_amount
                                                );
                                                info!("      (fee: {} USDT, net: {} USDT) - use pwap-web's multisig UI.",
                                                    fee as f64 / 1_000_000.0, net_amount as f64 / 1_000_000.0);

                                                db.add_deposit(DepositRecord {
                                                    id: db.next_deposit_id(),
                                                    polkadot_tx_hash: tx_hash.clone(),
                                                    polkadot_block: block_number as u64,
                                                    sender_address: from_str.clone(),
                                                    amount,
                                                    fee,
                                                    net_amount,
                                                    pezkuwi_tx_hash: None,
                                                    status: "pending_multisig_approval".to_string(),
                                                    created_at: now.to_rfc3339(),
                                                    processed_at: None,
                                                });
                                                db.save(&config.db_path)?;
                                                notify_telegram(config, &format!(
                                                    "📥 New deposit pending approval: {} USDT from {} -> mint {} wUSDT (net of fee) to {}. Sign at pwap-web's /multisig/pending.",
                                                    amount as f64 / 1_000_000.0, from_str, net_amount as f64 / 1_000_000.0, pezkuwi_recipient
                                                )).await;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("   ❌ Could not derive Pezkuwi recipient from sender {}: {}", from_str, e);
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

    Ok(())
}

/// Connects to `url` twice: once via the full subxt client (for reading storage/events/blocks),
/// once via the raw legacy RPC methods (needed only for `chain_get_block_hash` - resolving a
/// plain block NUMBER to a hash, which the high-level `Blocks` API doesn't expose directly). Two
/// lightweight connections to the same endpoint is simpler and safer than threading a shared
/// RPC client through `connect_to_chain`'s existing callers.
async fn connect_legacy_rpc(url: &str) -> Result<LegacyRpcMethods<SubstrateConfig>> {
    let rpc_client = RpcClient::from_url(url).await?;
    Ok(LegacyRpcMethods::<SubstrateConfig>::new(rpc_client))
}

/// Fetches block `block_number` by hash and runs it through `process_deposit_block`, then
/// persists the checkpoint. Used both for startup backfill (resuming after downtime) and for
/// closing any gap the live subscription itself might open up mid-run.
async fn backfill_deposit_block(
    config: &BridgeConfig,
    automation: &AutomationContext,
    client: &OnlineClient<SubstrateConfig>,
    legacy_rpc: &LegacyRpcMethods<SubstrateConfig>,
    block_number: u64,
) -> Result<()> {
    let hash = legacy_rpc
        .chain_get_block_hash(Some(block_number.into()))
        .await?
        .ok_or_else(|| anyhow!("no block hash found for Polkadot block #{block_number}"))?;
    let block = client.blocks().at(hash).await?;
    process_deposit_block(config, automation, &block).await?;

    let _guard = DB_LOCK.lock().await;
    let mut db = BridgeDatabase::load(&config.db_path)?;
    db.stats.last_polkadot_block = block_number;
    db.save(&config.db_path)?;
    Ok(())
}

async fn listen_deposits(config: &BridgeConfig, automation: &AutomationContext) -> Result<()> {
    let polkadot_addr = &config.custody_multisig_polkadot;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              DEPOSIT LISTENER (checkpoint + backfill)        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Bridge Address (Polkadot, 3-of-5 multisig): {}", polkadot_addr);
    println!("USDT Asset ID: {}", config.polkadot_usdt_asset_id);
    println!("Min Deposit: {} USDT", config.min_deposit as f64 / 1_000_000.0);
    println!("\nNo key held by this process can mint on the multisig's behalf -");
    println!("detected deposits are recorded as pending_multisig_approval. An actual");
    println!("mint requires 3 of the 5 signatories to approve via pwap-web.\n");

    // Outer loop: a full reconnect (fresh client + fresh legacy RPC) on every stall or dropped
    // subscription. Each pass re-derives last_seen from the persisted checkpoint and backfills any
    // gap before going live again - the same recovery path an ordinary process restart already
    // gets, just triggered by a detected stall instead of only by systemd.
    loop {
        let client = connect_to_chain(&config.polkadot_rpc).await?;
        let legacy_rpc = connect_legacy_rpc(&config.polkadot_rpc).await?;

        // Checkpoint + backfill: a live block subscription alone is not a correctness guarantee - a
        // transient reconnect or subscription hiccup can silently skip blocks with zero logging (this
        // is exactly how a real user deposit went undetected in production - see
        // the operator's custody notes 2026-07-15). On every (re)start, explicitly scan any gap between
        // the last persisted checkpoint and the current chain head before relying on the live stream.
        let last_processed = { BridgeDatabase::load(&config.db_path)?.stats.last_polkadot_block };
        let head_hash = legacy_rpc.chain_get_finalized_head().await?;
        let head_block = client.blocks().at(head_hash).await?;
        let head_number = head_block.number() as u64;

        let mut last_seen = if last_processed == 0 {
            info!("No checkpoint found - starting from current head (block {head_number}), not scanning full history");
            head_number
        } else if head_number > last_processed {
            info!("Backfilling Polkadot blocks {} to {head_number} before going live (resuming from checkpoint)", last_processed + 1);
            for n in (last_processed + 1)..=head_number {
                backfill_deposit_block(config, automation, &client, &legacy_rpc, n).await?;
                // A public/shared RPC endpoint (or a metered provider's free tier) can rate-limit
                // a tight burst of per-block calls hard enough to poison the whole connection
                // ("background task closed... restart required"), killing this entire listener
                // over what would otherwise be a routine catch-up after any real downtime -
                // observed live 2026-07-16. A small per-block pause costs little against a large
                // gap and avoids that failure mode entirely.
                tokio::time::sleep(Duration::from_millis(150)).await;
            }
            head_number
        } else {
            last_processed
        };

        {
            let _guard = DB_LOCK.lock().await;
            let mut db = BridgeDatabase::load(&config.db_path)?;
            db.stats.last_polkadot_block = last_seen;
            db.save(&config.db_path)?;
        }

        println!("Listening for deposits...\n");

        let mut blocks = client.blocks().subscribe_finalized().await?;

        loop {
            let next = match tokio::time::timeout(LIVE_SUBSCRIPTION_STALL_TIMEOUT, blocks.next()).await {
                Ok(next) => next,
                Err(_) => {
                    warn!("No new finalized Polkadot block in {}s - live subscription looks stalled, forcing a reconnect", LIVE_SUBSCRIPTION_STALL_TIMEOUT.as_secs());
                    notify_telegram(config, &format!(
                        "⚠️ usdt-bridge: Polkadot deposit subscription produced nothing for {}s - reconnecting now. Checkpoint backfill will catch up anything missed during the stall.",
                        LIVE_SUBSCRIPTION_STALL_TIMEOUT.as_secs()
                    )).await;
                    break;
                }
            };

            let Some(block) = next else {
                warn!("Polkadot block subscription ended - reconnecting");
                notify_telegram(config, "⚠️ usdt-bridge: Polkadot deposit subscription ended unexpectedly - reconnecting now.").await;
                break;
            };

            let block = match block {
                Ok(block) => block,
                Err(e) => {
                    warn!("Polkadot block subscription yielded an error ({e}) - reconnecting");
                    notify_telegram(config, &format!("⚠️ usdt-bridge: Polkadot deposit subscription errored ({e}) - reconnecting now.")).await;
                    break;
                }
            };

            let block_number = block.number() as u64;

            if block_number > last_seen + 1 {
                warn!("Gap detected in live Polkadot subscription: last processed {last_seen}, current {block_number} - backfilling {} missing block(s)", block_number - last_seen - 1);
                notify_telegram(config, &format!(
                    "⚠️ usdt-bridge: gap detected in live Polkadot block subscription ({} block(s) between #{last_seen} and #{block_number}) - backfilling now, no deposit should be lost, but this is worth knowing about.",
                    block_number - last_seen - 1
                )).await;
                for n in (last_seen + 1)..block_number {
                    backfill_deposit_block(config, automation, &client, &legacy_rpc, n).await?;
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
            }

            process_deposit_block(config, automation, &block).await?;
            last_seen = block_number;

            let _guard = DB_LOCK.lock().await;
            let mut db = BridgeDatabase::load(&config.db_path)?;
            db.stats.last_polkadot_block = block_number;
            db.save(&config.db_path)?;
        }
    }
}

/// Watches Pezkuwi Asset Hub for wUSDT sent to the bridge's own Pezkuwi account (a user
/// redeeming their wUSDT for real USDT) and completes the withdrawal automatically: idempotency
/// check, burn the received wUSDT to keep circulating supply honest, verify the Polkadot
/// custody wallet actually holds enough real USDT before releasing (the circuit-breaker this
/// bridge was missing - a mismatch here means the peg is under-collateralized and must halt,
/// not silently short-pay a user), then release on Polkadot. Mirrors listen_deposits() and
/// shares its event-matching caveat - see the comment there.
/// Processes exactly one block's withdrawal-relevant events - see process_deposit_block's doc
/// comment for why this is factored out identically for live and backfilled blocks.
async fn process_withdrawal_block(
    config: &BridgeConfig,
    automation: &AutomationContext,
    block: &subxt::blocks::Block<SubstrateConfig, OnlineClient<SubstrateConfig>>,
) -> Result<()> {
    let polkadot_bridge_addr = &config.custody_multisig_polkadot;
    let pezkuwi_bridge_addr = &config.custody_multisig_pezkuwi;
    let block_number = block.number();
    let events = block.events().await?;

    for event in events.iter() {
        let event = event?;

        if event.pallet_name() == "Assets" && event.variant_name() == "Transferred" {
                if let Ok(fields) = event.field_values() {
                    let asset_id = fields.at("asset_id")
                        .and_then(|v| v.as_u128())
                        .unwrap_or(0) as u32;

                    if asset_id != config.pezkuwi_wusdt_asset_id {
                        continue;
                    }

                    let to_field = fields.at("to");
                    let from_field = fields.at("from");
                    let amount = fields.at("amount")
                        .and_then(|v| v.as_u128())
                        .unwrap_or(0);

                    let target_bytes = sp_core::crypto::AccountId32::from_ss58check(pezkuwi_bridge_addr)
                        .map(|a| Into::<[u8; 32]>::into(a))
                        .ok();
                    let to_bytes = to_field.and_then(account_id_from_dynamic);

                    if to_bytes.is_some() && to_bytes == target_bytes {
                        {
                            let from_bytes_opt = from_field.and_then(account_id_from_dynamic);
                            let from_str = from_bytes_opt.map(|b| account_bytes_to_ss58(b, 42)).unwrap_or_default();
                            let tx_hash = format!("block-{}-{}", block_number, from_str);

                            info!("📤 WITHDRAWAL DETECTED!");
                            info!("   Block: #{}", block_number);
                            info!("   From: {}", from_str);
                            info!("   Amount: {} USDT", amount as f64 / 1_000_000.0);

                            let _guard = DB_LOCK.lock().await;
                            let mut db = BridgeDatabase::load(&config.db_path)?;

                            if db.is_withdrawal_processed(&tx_hash) {
                                warn!("   ⚠️ Already processed ({}), skipping - refusing to release twice", tx_hash);
                                continue;
                            }

                            if amount < config.min_withdraw {
                                warn!("   ⚠️ Amount below minimum, skipping");
                                continue;
                            }

                            let fee = calculate_fee(amount, config.fee_basis_points);
                            let net_amount = amount - fee;

                            if amount > config.max_single_tx {
                                warn!(
                                    "   ⚠️ Amount {} exceeds max_single_tx cap {} - recording as pending_multisig_approval_amount_exceeds_cap, needs manual multi-party review",
                                    amount as f64 / 1_000_000.0,
                                    config.max_single_tx as f64 / 1_000_000.0
                                );
                                notify_telegram(config, &format!(
                                    "📤 Withdrawal {} USDT from {} exceeds max_single_tx ({} USDT) - recorded, needs manual multi-party review.",
                                    amount as f64 / 1_000_000.0, from_str, config.max_single_tx as f64 / 1_000_000.0
                                )).await;
                                let destination_address = convert_ss58_prefix(&from_str, 0).unwrap_or_default();
                                db.add_withdrawal(WithdrawalRecord {
                                    id: db.next_withdrawal_id(),
                                    pezkuwi_tx_hash: tx_hash.clone(),
                                    pezkuwi_block: block_number as u64,
                                    sender_address: from_str.clone(),
                                    destination_address,
                                    amount,
                                    fee,
                                    net_amount,
                                    polkadot_tx_hash: None,
                                    status: "pending_multisig_approval_amount_exceeds_cap".to_string(),
                                    created_at: Utc::now().to_rfc3339(),
                                    processed_at: None,
                                });
                                db.save(&config.db_path)?;
                                continue;
                            }

                            // Circuit breaker: never release more real USDT than the custody
                            // wallet actually holds. If this ever trips, the peg is already
                            // under-collateralized (e.g. wUSDT minted without a matching real
                            // deposit) - that must halt and page someone, not degrade silently.
                            //
                            // Note this reserve check is best-effort only: the withdrawal itself
                            // is already finalized on Pezkuwi by the time we get here, so a
                            // transient RPC hiccup while trying to verify the Polkadot-side
                            // reserve must NEVER `continue` past recording it - that would drop
                            // a real, already-committed withdrawal on the floor with no
                            // pending_multisig_approval entry anywhere (exactly the silent-loss
                            // failure mode fixed for the subscription-stall bug). The signatories
                            // verify the real reserve themselves in pwap-web before approving, so
                            // an unverified reserve here just means "never auto-pay, always
                            // human-review" - it does not block recording the request.
                            let polkadot_client = connect_to_chain(&config.polkadot_rpc).await.ok();
                            let reserve_known = polkadot_client.is_some();

                            let reserve = match &polkadot_client {
                                Some(client) => get_asset_balance(
                                    client,
                                    config.polkadot_usdt_asset_id,
                                    polkadot_bridge_addr,
                                ).await.unwrap_or(0),
                                None => 0,
                            };

                            if !reserve_known {
                                error!("   ❌ Failed to connect to Polkadot to verify reserve - recording withdrawal as pending_multisig_approval anyway (reserve NOT confirmed, signatories must check manually)");
                                notify_telegram(config, &format!(
                                    "⚠️ Could not verify Polkadot reserve for a {} USDT withdrawal from {} (RPC connect failed). Recorded as pending_multisig_approval - reserve NOT confirmed, signatories must check manually before approving.",
                                    amount as f64 / 1_000_000.0, from_str
                                )).await;
                            } else if reserve < net_amount {
                                let shortfall_msg = format!(
                                    "🚨 RESERVE SHORTFALL: custody holds {} USDT, need {} USDT to honor a pending {} USDT withdrawal from {}. \
                                     This must be reviewed by the signatories before any approval - do NOT approve as-is.",
                                    reserve as f64 / 1_000_000.0,
                                    net_amount as f64 / 1_000_000.0,
                                    amount as f64 / 1_000_000.0,
                                    from_str,
                                );
                                notify_telegram(config, &shortfall_msg).await;
                            }

                            match convert_ss58_prefix(&from_str, 0) {
                                Ok(polkadot_recipient) => {
                                    let now = Utc::now();
                                    // Never attempt auto-pay against an unverified or known-short reserve -
                                    // both cases must always go to human review.
                                    let auto_pay_attempt = if !reserve_known || reserve < net_amount {
                                        None
                                    } else {
                                        match (from_bytes_opt, &automation.polkadot_signing_client, &automation.polkadot_keypair) {
                                            (Some(dest_bytes), Some(client), Some(keypair)) => {
                                                match check_auto_pay_eligible(&db, config, LEG_WITHDRAWAL_USDT_PAYOUT, net_amount, now) {
                                                    Ok(()) => {
                                                        let owner_bytes = sp_core::crypto::AccountId32::from_ss58check(polkadot_bridge_addr)
                                                            .map(Into::<[u8; 32]>::into)
                                                            .map_err(|e| format!("bad custody_multisig_polkadot config: {e:?}"));
                                                        match owner_bytes {
                                                            Ok(owner_bytes) => {
                                                                let first = execute_usdt_auto_payout(client, keypair, config.polkadot_usdt_asset_id, owner_bytes, dest_bytes, net_amount).await;
                                                                // Same reconnect-and-retry as the wUSDT leg - see its comment. This
                                                                // exact failure ("Connection reset by peer... restart required")
                                                                // was observed live twice on this leg before this fix.
                                                                let result = if first.is_err() {
                                                                    warn!("   ⚠️ Auto-pay attempt failed ({:?}) - retrying once with a fresh connection", first.as_ref().err());
                                                                    match OnlineClient::<SubstrateConfig>::from_url(&config.polkadot_rpc).await {
                                                                        Ok(fresh_client) => execute_usdt_auto_payout(&fresh_client, keypair, config.polkadot_usdt_asset_id, owner_bytes, dest_bytes, net_amount).await,
                                                                        Err(_) => first,
                                                                    }
                                                                } else {
                                                                    first
                                                                };
                                                                Some(result)
                                                            }
                                                            Err(reason) => { warn!("   ⚠️ Auto-pay skipped: {reason}"); None }
                                                        }
                                                    }
                                                    Err(reason) => { info!("   ℹ️ Not auto-paying (falls back to manual): {reason}"); None }
                                                }
                                            }
                                            _ => None,
                                        }
                                    };

                                    match auto_pay_attempt {
                                        Some(Ok(polkadot_tx_hash)) => {
                                            // The wUSDT the user sent in is NOT burned here - burn is
                                            // Owner/Admin-only (multisig-only, no delegation exists for
                                            // it). Paying the user out is what's time-critical; the
                                            // matching burn is deferred bookkeeping the signatories batch
                                            // periodically via pwap-web - tracked explicitly by this
                                            // status so it can never be silently forgotten.
                                            info!("   ✅ AUTO-PAID: {} USDT -> {} (tx {}); wUSDT burn still pending (multisig-only)", net_amount as f64 / 1_000_000.0, polkadot_recipient, polkadot_tx_hash);
                                            db.record_auto_payout(LEG_WITHDRAWAL_USDT_PAYOUT, net_amount, now);
                                            let breaker_tripped = maybe_trip_circuit_breaker(&mut db, config, LEG_WITHDRAWAL_USDT_PAYOUT, now);
                                            db.add_withdrawal(WithdrawalRecord {
                                                id: db.next_withdrawal_id(),
                                                pezkuwi_tx_hash: tx_hash.clone(),
                                                pezkuwi_block: block_number as u64,
                                                sender_address: from_str.clone(),
                                                destination_address: polkadot_recipient.clone(),
                                                amount, fee, net_amount,
                                                polkadot_tx_hash: Some(polkadot_tx_hash.clone()),
                                                status: "auto_paid_burn_pending".to_string(),
                                                created_at: now.to_rfc3339(),
                                                processed_at: Some(now.to_rfc3339()),
                                            });
                                            db.save(&config.db_path)?;
                                            if let Some(reason) = breaker_tripped {
                                                notify_telegram(config, &format!("🚨 CIRCUIT BREAKER TRIPPED (withdrawal/USDT leg): {reason}. All further withdrawals now queue for manual 3-of-5 review until `reset-circuit-breaker` is run after human review.")).await;
                                            } else if amount >= config.notify_threshold {
                                                notify_telegram(config, &format!(
                                                    "✅ Auto-paid withdrawal: {} wUSDT from {} -> {} USDT (net of fee) to {}. tx {}. wUSDT burn still pending - batch via pwap-web when convenient.",
                                                    amount as f64 / 1_000_000.0, from_str, net_amount as f64 / 1_000_000.0, polkadot_recipient, polkadot_tx_hash
                                                )).await;
                                            }
                                        }
                                        Some(Err(e)) => {
                                            error!("   ❌ Auto-pay execution FAILED, falling back to manual review: {e:#}");
                                            notify_telegram(config, &format!(
                                                "⚠️ Auto-pay attempt failed for withdrawal {} USDT from {} - falling back to manual 3-of-5 review. Error: {e:#}",
                                                amount as f64 / 1_000_000.0, from_str
                                            )).await;
                                            db.add_withdrawal(WithdrawalRecord {
                                                id: db.next_withdrawal_id(),
                                                pezkuwi_tx_hash: tx_hash.clone(),
                                                pezkuwi_block: block_number as u64,
                                                sender_address: from_str.clone(),
                                                destination_address: polkadot_recipient,
                                                amount, fee, net_amount,
                                                polkadot_tx_hash: None,
                                                status: "pending_multisig_approval".to_string(),
                                                created_at: now.to_rfc3339(),
                                                processed_at: None,
                                            });
                                            db.save(&config.db_path)?;
                                        }
                                        None => {
                                            info!("   ⏳ AWAITING MULTISIG APPROVAL - not auto-burning/releasing.");
                                            info!(
                                                "      3 of 5 signatories must approve: burn {} wUSDT, then Assets.transfer({}, {}, {}) on Polkadot",
                                                amount as f64 / 1_000_000.0, config.polkadot_usdt_asset_id, polkadot_recipient, net_amount
                                            );
                                            if reserve_known {
                                                info!("      Reserve check at detection time: {} USDT available. Use pwap-web's multisig UI.",
                                                    reserve as f64 / 1_000_000.0);
                                            } else {
                                                info!("      Reserve could NOT be verified (Polkadot RPC unreachable at detection time) - signatories must check manually via pwap-web.");
                                            }

                                            db.add_withdrawal(WithdrawalRecord {
                                                id: db.next_withdrawal_id(),
                                                pezkuwi_tx_hash: tx_hash.clone(),
                                                pezkuwi_block: block_number as u64,
                                                sender_address: from_str.clone(),
                                                destination_address: polkadot_recipient.clone(),
                                                amount,
                                                fee,
                                                net_amount,
                                                polkadot_tx_hash: None,
                                                status: if !reserve_known {
                                                    "pending_multisig_approval_reserve_unverified".to_string()
                                                } else if reserve < net_amount {
                                                    "pending_multisig_approval_reserve_shortfall".to_string()
                                                } else {
                                                    "pending_multisig_approval".to_string()
                                                },
                                                created_at: now.to_rfc3339(),
                                                processed_at: None,
                                            });
                                            db.save(&config.db_path)?;

                                            // Unlike the two branches above (reserve_unverified/reserve_shortfall already
                                            // notify further up), the plain "automation not configured/eligible" case had no
                                            // notification at all until now - a withdrawal needing manual 3-of-5 review must
                                            // always page someone, the same as the deposit leg already does.
                                            if reserve_known && reserve >= net_amount {
                                                notify_telegram(config, &format!(
                                                    "📤 New withdrawal pending approval: {} USDT from {} -> release {} USDT (net of fee) to {}. Sign at pwap-web's /multisig/pending.",
                                                    amount as f64 / 1_000_000.0, from_str, net_amount as f64 / 1_000_000.0, polkadot_recipient
                                                )).await;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("   ❌ Could not derive Polkadot recipient from sender {}: {}", from_str, e);
                                }
                            }
                        }
                    }
                }
            }
        }

    Ok(())
}

/// Same pattern as backfill_deposit_block, for the Pezkuwi side.
async fn backfill_withdrawal_block(
    config: &BridgeConfig,
    automation: &AutomationContext,
    client: &OnlineClient<SubstrateConfig>,
    legacy_rpc: &LegacyRpcMethods<SubstrateConfig>,
    block_number: u64,
) -> Result<()> {
    let hash = legacy_rpc
        .chain_get_block_hash(Some(block_number.into()))
        .await?
        .ok_or_else(|| anyhow!("no block hash found for Pezkuwi block #{block_number}"))?;
    let block = client.blocks().at(hash).await?;
    process_withdrawal_block(config, automation, &block).await?;

    let _guard = DB_LOCK.lock().await;
    let mut db = BridgeDatabase::load(&config.db_path)?;
    db.stats.last_pezkuwi_block = block_number;
    db.save(&config.db_path)?;
    Ok(())
}

async fn listen_withdrawals(config: &BridgeConfig, automation: &AutomationContext) -> Result<()> {
    let pezkuwi_bridge_addr = &config.custody_multisig_pezkuwi;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              WITHDRAWAL LISTENER (checkpoint + backfill)     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Bridge Address (Pezkuwi, 3-of-5 multisig): {}", pezkuwi_bridge_addr);
    println!("wUSDT Asset ID: {}", config.pezkuwi_wusdt_asset_id);
    println!("Min Withdraw: {} USDT", config.min_withdraw as f64 / 1_000_000.0);
    println!("\nNo key held by this process can burn/release on the multisig's behalf -");
    println!("detected withdrawal requests are recorded as pending_multisig_approval.\n");

    // Outer loop: same stall-recovery strategy as listen_deposits - see its comment for why a
    // live subscription alone (even one that's never restarted) isn't a correctness guarantee.
    loop {
        let pezkuwi_client = connect_to_chain(&config.pezkuwi_rpc).await?;
        let legacy_rpc = connect_legacy_rpc(&config.pezkuwi_rpc).await?;

        // Checkpoint + backfill - see listen_deposits for why this replaced pure live-subscription.
        let last_processed = { BridgeDatabase::load(&config.db_path)?.stats.last_pezkuwi_block };
        let head_hash = legacy_rpc.chain_get_finalized_head().await?;
        let head_block = pezkuwi_client.blocks().at(head_hash).await?;
        let head_number = head_block.number() as u64;

        let mut last_seen = if last_processed == 0 {
            info!("No checkpoint found - starting from current head (block {head_number}), not scanning full history");
            head_number
        } else if head_number > last_processed {
            info!("Backfilling Pezkuwi blocks {} to {head_number} before going live (resuming from checkpoint)", last_processed + 1);
            for n in (last_processed + 1)..=head_number {
                backfill_withdrawal_block(config, automation, &pezkuwi_client, &legacy_rpc, n).await?;
            }
            head_number
        } else {
            last_processed
        };

        {
            let _guard = DB_LOCK.lock().await;
            let mut db = BridgeDatabase::load(&config.db_path)?;
            db.stats.last_pezkuwi_block = last_seen;
            db.save(&config.db_path)?;
        }

        println!("Listening for withdrawals...\n");

        let mut blocks = pezkuwi_client.blocks().subscribe_finalized().await?;

        loop {
            let next = match tokio::time::timeout(LIVE_SUBSCRIPTION_STALL_TIMEOUT, blocks.next()).await {
                Ok(next) => next,
                Err(_) => {
                    warn!("No new finalized Pezkuwi block in {}s - live subscription looks stalled, forcing a reconnect", LIVE_SUBSCRIPTION_STALL_TIMEOUT.as_secs());
                    notify_telegram(config, &format!(
                        "⚠️ usdt-bridge: Pezkuwi withdrawal subscription produced nothing for {}s - reconnecting now. Checkpoint backfill will catch up anything missed during the stall.",
                        LIVE_SUBSCRIPTION_STALL_TIMEOUT.as_secs()
                    )).await;
                    break;
                }
            };

            let Some(block) = next else {
                warn!("Pezkuwi block subscription ended - reconnecting");
                notify_telegram(config, "⚠️ usdt-bridge: Pezkuwi withdrawal subscription ended unexpectedly - reconnecting now.").await;
                break;
            };

            let block = match block {
                Ok(block) => block,
                Err(e) => {
                    warn!("Pezkuwi block subscription yielded an error ({e}) - reconnecting");
                    notify_telegram(config, &format!("⚠️ usdt-bridge: Pezkuwi withdrawal subscription errored ({e}) - reconnecting now.")).await;
                    break;
                }
            };

            let block_number = block.number() as u64;

            if block_number > last_seen + 1 {
                warn!("Gap detected in live Pezkuwi subscription: last processed {last_seen}, current {block_number} - backfilling {} missing block(s)", block_number - last_seen - 1);
                notify_telegram(config, &format!(
                    "⚠️ usdt-bridge: gap detected in live Pezkuwi block subscription ({} block(s) between #{last_seen} and #{block_number}) - backfilling now, no withdrawal should be lost, but this is worth knowing about.",
                    block_number - last_seen - 1
                )).await;
                for n in (last_seen + 1)..block_number {
                    backfill_withdrawal_block(config, automation, &pezkuwi_client, &legacy_rpc, n).await?;
                }
            }

            process_withdrawal_block(config, automation, &block).await?;
            last_seen = block_number;

            let _guard = DB_LOCK.lock().await;
            let mut db = BridgeDatabase::load(&config.db_path)?;
            db.stats.last_pezkuwi_block = block_number;
            db.save(&config.db_path)?;
        }
    }
}

async fn process_withdrawals(config: &BridgeConfig) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              WITHDRAWAL STATUS (read-only)                   ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Live withdrawal processing now runs automatically via 'listen-withdrawals'");
    println!("(or 'listen-all', which runs both directions concurrently). This command");
    println!("only reports what's recorded in the database so far.\n");

    // Load database
    let db = BridgeDatabase::load(&config.db_path)?;

    let pending: Vec<_> = db.withdrawals.iter()
        .filter(|w| w.status != "completed")
        .collect();

    if pending.is_empty() {
        println!("No pending/failed withdrawals.");
    } else {
        println!("Non-completed withdrawals: {}", pending.len());
        for w in pending {
            println!("  #{} [{}]: {} USDT -> {}", w.id, w.status, w.amount as f64 / 1_000_000.0, w.destination_address);
        }
    }

    Ok(())
}

async fn listen_all(config: &BridgeConfig) -> Result<()> {
    let automation = init_automation_context(config).await;
    tokio::try_join!(listen_deposits(config, &automation), listen_withdrawals(config, &automation))?;
    Ok(())
}

async fn show_automation_status(config: &BridgeConfig) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              AUTOMATION KEY AUTO-PAY STATUS                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("Per-tx cap (max_single_tx):     {} USDT", config.max_single_tx as f64 / 1_000_000.0);
    println!("Daily cap (per leg):            {} USDT", config.auto_pay_daily_cap as f64 / 1_000_000.0);
    println!("Circuit breaker:                 >= {:.0}% of daily cap within {} min\n",
        config.circuit_breaker_fraction * 100.0, config.circuit_breaker_window_minutes);

    let db = BridgeDatabase::load(&config.db_path)?;
    let now = Utc::now();

    for (label, leg) in [("Deposits (wUSDT payout)", LEG_DEPOSIT_WUSDT_PAYOUT), ("Withdrawals (USDT payout)", LEG_WITHDRAWAL_USDT_PAYOUT)] {
        let vol_24h = db.recent_auto_payout_volume(leg, now, 24 * 60);
        println!("{label}:");
        println!("  Rolling 24h auto-paid: {} / {} USDT", vol_24h as f64 / 1_000_000.0, config.auto_pay_daily_cap as f64 / 1_000_000.0);
    }

    println!();
    if db.circuit_breaker_tripped {
        println!("🚨 CIRCUIT BREAKER: TRIPPED at {}", db.circuit_breaker_tripped_at.as_deref().unwrap_or("unknown"));
        println!("   Reason: {}", db.circuit_breaker_reason.as_deref().unwrap_or("none recorded"));
        println!("   Run `reset-circuit-breaker` after human review to resume auto-pay.");
    } else {
        println!("✅ Circuit breaker: clear");
    }

    if std::env::var("AUTOMATION_KEY_MNEMONIC").is_err() {
        println!("\n⚠️ AUTOMATION_KEY_MNEMONIC not set in this environment - auto-pay is fully disabled, everything falls back to manual 3-of-5 review.");
    }

    Ok(())
}

fn reset_circuit_breaker(config: &BridgeConfig) -> Result<()> {
    let mut db = BridgeDatabase::load(&config.db_path)?;
    if !db.circuit_breaker_tripped {
        println!("Circuit breaker was not tripped - nothing to do.");
        return Ok(());
    }
    println!("Clearing circuit breaker (was: {})", db.circuit_breaker_reason.as_deref().unwrap_or("no reason recorded"));
    db.circuit_breaker_tripped = false;
    db.circuit_breaker_reason = None;
    db.circuit_breaker_tripped_at = None;
    db.save(&config.db_path)?;
    println!("Circuit breaker cleared - auto-pay will resume on the next eligible event.");
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
    let mut config: BridgeConfig = if cli.config.exists() {
        let content = std::fs::read_to_string(&cli.config)?;
        serde_json::from_str(&content)?
    } else {
        let default_config = BridgeConfig::default();
        let json = serde_json::to_string_pretty(&default_config)?;
        std::fs::write(&cli.config, &json)?;
        info!("Created default config at: {}", cli.config.display());
        default_config
    };

    // Dedicated/API-keyed RPC endpoints (e.g. a Dwellir URL) carry a credential in the URL itself,
    // so - same reasoning as PEZBRIDGE_TELEGRAM_TOKEN/AUTOMATION_KEY_MNEMONIC below - they belong
    // in an env var backed by a tightly-permissioned EnvironmentFile, not in the git-tracked JSON
    // config. Unset means "use whatever bridge_config.json says" (the public endpoint by default).
    if let Ok(url) = std::env::var("POLKADOT_RPC_URL") {
        config.polkadot_rpc = url;
    }
    if let Ok(url) = std::env::var("PEZKUWI_RPC_URL") {
        config.pezkuwi_rpc = url;
    }

    match cli.command {
        Commands::GenerateWallet { output } => {
            generate_wallet(&output)?;
        }
        Commands::ShowAddresses { seed } => {
            show_addresses(&seed)?;
        }
        Commands::ListenDeposits => {
            let automation = init_automation_context(&config).await;
            listen_deposits(&config, &automation).await?;
        }
        Commands::ListenWithdrawals => {
            let automation = init_automation_context(&config).await;
            listen_withdrawals(&config, &automation).await?;
        }
        Commands::ListenAll => {
            listen_all(&config).await?;
        }
        Commands::ProcessWithdrawals => {
            process_withdrawals(&config).await?;
        }
        Commands::Reserves => {
            show_reserves(&config).await?;
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
        Commands::AutomationStatus => {
            show_automation_status(&config).await?;
        }
        Commands::ResetCircuitBreaker => {
            reset_circuit_breaker(&config)?;
        }
    }

    Ok(())
}
