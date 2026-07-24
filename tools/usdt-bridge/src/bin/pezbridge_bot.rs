//! PezbridgeBot - notifies signers when the USDT bridge automation key's `Assets.approve_transfer`
//! spending allowance needs renewal (see the operator's custody notes and the
//! usdt-bridge-multisig-migration memory for the full custody model).
//!
//! This process holds NO private key and cannot sign or submit anything. An earlier version of
//! this bot held Noter's (Validator_04's) key so it could automate 1-of-3 required signatures -
//! that made sense when it was written, since pwap-web had no working signing UI at all and
//! every approval required a hand-written script. It stopped making sense once pwap-web's real
//! multisig operations UI shipped (shared/lib/multisig.ts + web/src/components/
//! MultisigOperations.tsx, at /multisig/pending): any of the 5 signers can now approve directly
//! with their own wallet extension, so the only thing a bot-held key still bought was "2 humans
//! click instead of 3" - not worth a private key sitting on a server for. Simplified back down
//! to detect-and-notify only; Noter's mnemonic was removed from this host entirely.
//!
//! Flow:
//! 1. Poll the automation key's remaining Assets.Approvals allowance for wUSDT (asset 1000).
//! 2. When it drops below `renewal_threshold`, send ONE Telegram notification (to Serok's chat
//!    and the ops group) pointing signers at pwap-web's /multisig/pending page, where any 3 of
//!    the 5 real signatories can approve the renewal with their own wallets.
//! 3. Suppress repeat notifications until the allowance is actually renewed (goes back above
//!    threshold) - avoids spamming every poll cycle while humans coordinate.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sp_core::crypto::Ss58Codec;
use std::path::PathBuf;
use std::time::Duration;
use subxt::dynamic::{At, Value};
use subxt::{OnlineClient, SubstrateConfig};
use tracing::{error, info, warn};

const WUSDT_ASSET_ID: u32 = 1000;
const MULTISIG_ADDR: &str = "5GvwxmCDp3PC33KHoeWSgj3S7ocE7nzk1jiCCZMPSDBFeNcj";
const AUTOMATION_KEY_ADDR: &str = "5GQu4PFUb1f3MTJ7i7c1CtLgDk3TVvpSW1VbQCRmfkMoC8cM";
const MULTISIG_THRESHOLD: u16 = 3;
const PWAP_WEB_OPERATIONS_URL: &str = "https://app.pezkuwichain.io/multisig/pending";

// ============================================================================
// Config / state
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BotConfig {
    pezkuwi_rpc: String,
    serok_chat_id: i64,
    ops_chat_id: i64,
    poll_interval_secs: u64,
    renewal_threshold: u128,
    topup_amount: u128,
    state_path: PathBuf,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            pezkuwi_rpc: "wss://asset-hub-rpc.pezkuwichain.io".to_string(),
            serok_chat_id: 0,
            ops_chat_id: 0,
            poll_interval_secs: 600,
            renewal_threshold: 3_000_000_000, // 3,000 wUSDT
            topup_amount: 10_000_000_000,     // 10,000 wUSDT
            state_path: PathBuf::from("pezbridge_bot_state.json"),
        }
    }
}

fn load_config() -> Result<BotConfig> {
    let path = std::env::var("PEZBRIDGE_CONFIG").unwrap_or_else(|_| "pezbridge_bot_config.json".to_string());
    if std::path::Path::new(&path).exists() {
        let content = std::fs::read_to_string(&path).context("reading config file")?;
        Ok(serde_json::from_str(&content).context("parsing config file")?)
    } else {
        warn!("No config file at {path}, using defaults - serok_chat_id/ops_chat_id MUST be set for this to be useful");
        Ok(BotConfig::default())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct BotState {
    /// Whether a notification has already been sent for the CURRENT low-allowance episode -
    /// reset back to false once the allowance rises back above threshold (meaning humans
    /// renewed it via pwap-web), so the next depletion gets its own fresh notification.
    already_notified: bool,
}

fn load_state(path: &PathBuf) -> Result<BotState> {
    if path.exists() {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content).unwrap_or_default())
    } else {
        Ok(BotState::default())
    }
}

fn save_state(path: &PathBuf, state: &BotState) -> Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

// ============================================================================
// Chain read
// ============================================================================

async fn get_approval_remaining(
    client: &OnlineClient<SubstrateConfig>,
    asset_id: u32,
    owner: &str,
    delegate: &str,
) -> Result<u128> {
    let owner_bytes = sp_core::crypto::AccountId32::from_ss58check(owner)
        .map_err(|e| anyhow!("invalid owner address: {e:?}"))?;
    let delegate_bytes = sp_core::crypto::AccountId32::from_ss58check(delegate)
        .map_err(|e| anyhow!("invalid delegate address: {e:?}"))?;

    let storage_query = subxt::dynamic::storage(
        "Assets",
        "Approvals",
        vec![
            Value::primitive(asset_id.into()),
            Value::from_bytes(<sp_core::crypto::AccountId32 as AsRef<[u8; 32]>>::as_ref(&owner_bytes)),
            Value::from_bytes(<sp_core::crypto::AccountId32 as AsRef<[u8; 32]>>::as_ref(&delegate_bytes)),
        ],
    );

    let result = client.storage().at_latest().await?.fetch(&storage_query).await?;
    if let Some(v) = result {
        if let Some(amount) = v.to_value()?.at("amount").and_then(|a| a.as_u128()) {
            return Ok(amount);
        }
    }
    // No approval entry at all = the allowance is fully exhausted (or was never granted) -
    // treat identically to "0 remaining" so it triggers a notification.
    Ok(0)
}

// ============================================================================
// Telegram (send-only - no polling/callbacks needed anymore, there's no action this bot takes)
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

async fn notify(http: &reqwest::Client, token: &str, config: &BotConfig, msg: &str) {
    info!("[pezbridge-bot] {msg}");
    if let Err(e) = tg_send_message(http, token, config.serok_chat_id, msg).await {
        error!("failed to notify Serok: {e:#}");
    }
    if let Err(e) = tg_send_message(http, token, config.ops_chat_id, msg).await {
        error!("failed to notify ops chat: {e:#}");
    }
}

// ============================================================================
// Core logic
// ============================================================================

async fn check_allowance_and_maybe_notify(
    read_client: &OnlineClient<SubstrateConfig>,
    http: &reqwest::Client,
    token: &str,
    config: &BotConfig,
    state: &mut BotState,
) -> Result<()> {
    let remaining = get_approval_remaining(read_client, WUSDT_ASSET_ID, MULTISIG_ADDR, AUTOMATION_KEY_ADDR).await?;
    info!("Automation key remaining allowance: {:.2} wUSDT", remaining as f64 / 1_000_000.0);

    if remaining >= config.renewal_threshold {
        if state.already_notified {
            info!("Allowance is back above threshold - renewal happened, resetting notification state");
            state.already_notified = false;
            save_state(&config.state_path, state)?;
        }
        return Ok(());
    }

    if state.already_notified {
        return Ok(()); // already told everyone, don't spam while they coordinate
    }

    let topup = config.topup_amount;
    let text = format!(
        "\u{26A0}\u{FE0F} USDT Bridge - otomasyon anahtarı izni azaldı\n\n\
         Kalan izin: {:.2} wUSDT\nEşik: {:.2} wUSDT\n\n\
         Önerilen yenileme: {:.2} wUSDT (Assets.approve_transfer, multisig'den otomasyon anahtarına).\n\n\
         {} imzacının (5'ten herhangi 3'ü) pwap-web üzerinden kendi cüzdanlarıyla onaylaması gerekiyor:\n{}",
        remaining as f64 / 1_000_000.0,
        config.renewal_threshold as f64 / 1_000_000.0,
        topup as f64 / 1_000_000.0,
        MULTISIG_THRESHOLD,
        PWAP_WEB_OPERATIONS_URL
    );

    notify(http, token, config, &text).await;

    state.already_notified = true;
    save_state(&config.state_path, state)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();

    let config = load_config()?;
    if config.serok_chat_id == 0 && config.ops_chat_id == 0 {
        return Err(anyhow!("both serok_chat_id and ops_chat_id are unset - this bot would notify no one"));
    }

    let token = std::env::var("PEZBRIDGE_TELEGRAM_TOKEN").context("PEZBRIDGE_TELEGRAM_TOKEN env var not set")?;

    info!("Connecting to {}...", config.pezkuwi_rpc);
    let read_client = OnlineClient::<SubstrateConfig>::from_url(&config.pezkuwi_rpc)
        .await
        .context("failed to connect")?;

    let http = reqwest::Client::new();
    let mut state = load_state(&config.state_path)?;

    notify(&http, &token, &config, "PezbridgeBot started (notify-only, holds no key).").await;

    let mut poll_timer = tokio::time::interval(Duration::from_secs(config.poll_interval_secs));
    loop {
        poll_timer.tick().await;
        if let Err(e) = check_allowance_and_maybe_notify(&read_client, &http, &token, &config, &mut state).await {
            error!("allowance check failed: {e:#}");
        }
    }
}
