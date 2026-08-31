//! Move wUSDT bridge custody from Treasury_1 to the new 3-of-5 multisig, in one atomic batch:
//!   1. Transfer 1,000,000 wUSDT from Treasury_1 to the multisig (seed liquidity pool)
//!   2. set_team(issuer=admin=freezer=multisig)
//!   3. transfer_ownership(owner=multisig)   <- must be last: revokes Treasury_1's owner rights
//!
//! Signed by Treasury_1 (verified match against the known Treasury_1 address before running).

#![allow(missing_docs, dead_code)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const WUSDT_ASSET_ID: u32 = 1000;
const MULTISIG_ADDR: &str = "5GvwxmCDp3PC33KHoeWSgj3S7ocE7nzk1jiCCZMPSDBFeNcj";
const SEED_AMOUNT: u128 = 1_000_000_000_000; // 1,000,000 wUSDT (6 decimals)

fn multi_address_id(account: &AccountId32) -> Value {
    Value::unnamed_variant("Id", [Value::from_bytes(account.0)])
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MIGRATE wUSDT CUSTODY: Treasury_1 -> 3-of-5 multisig ===\n");

    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "wss://asset-hub-rpc.pezkuwichain.io".to_string());

    let multisig: AccountId32 = MULTISIG_ADDR.parse()?;
    println!("Multisig target: {}", MULTISIG_ADDR);
    println!("Seed amount: {} wUSDT\n", SEED_AMOUNT as f64 / 1_000_000.0);

    let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&rpc_url).await?;
    println!("Connected to {}! specVersion: {}\n", rpc_url, api.runtime_version().spec_version);

    let seed_content = std::fs::read_to_string("treasury.json")?;
    let seed_json: serde_json::Value = serde_json::from_str(&seed_content)?;
    let mnemonic_str = seed_json["mnemonic"].as_str().expect("mnemonic field present");
    let mnemonic = Mnemonic::from_str(mnemonic_str)?;
    let treasury_keypair = Keypair::from_phrase(&mnemonic, None)?;
    let treasury_account = treasury_keypair.public_key().to_account_id();
    println!("Signer (Treasury_1): {}", treasury_account);
    assert_eq!(
        treasury_account.to_string(),
        "5EhCpn82QtdU53MF6PoNFrKHgSrsfcAxFTMwrn3JYf9dioQw",
        "Refusing to proceed: signer does not match known Treasury_1 address"
    );

    let transfer_call = pezkuwi_subxt::dynamic::tx(
        "Assets",
        "transfer",
        vec![
            Value::primitive(WUSDT_ASSET_ID.into()),
            multi_address_id(&multisig),
            Value::u128(SEED_AMOUNT),
        ],
    );

    let set_team_call = pezkuwi_subxt::dynamic::tx(
        "Assets",
        "set_team",
        vec![
            Value::primitive(WUSDT_ASSET_ID.into()),
            multi_address_id(&multisig),
            multi_address_id(&multisig),
            multi_address_id(&multisig),
        ],
    );

    let transfer_ownership_call = pezkuwi_subxt::dynamic::tx(
        "Assets",
        "transfer_ownership",
        vec![Value::primitive(WUSDT_ASSET_ID.into()), multi_address_id(&multisig)],
    );

    let batch_call = pezkuwi_subxt::dynamic::tx(
        "Utility",
        "batch_all",
        vec![Value::unnamed_composite(vec![
            transfer_call.into_value(),
            set_team_call.into_value(),
            transfer_ownership_call.into_value(),
        ])],
    );

    println!("Submitting batch_all(transfer 1M wUSDT, set_team, transfer_ownership)...");

    use pezkuwi_subxt::tx::TxStatus;
    let tx_progress =
        api.tx().sign_and_submit_then_watch_default(&batch_call, &treasury_keypair).await?;

    println!("TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

    let mut progress = tx_progress;
    loop {
        match progress.next().await {
            Some(Ok(TxStatus::InBestBlock(details))) => {
                match details.wait_for_success().await {
                    Ok(events) => {
                        let mut has_batch_completed = false;
                        for ev in events.iter().flatten() {
                            println!("  event: {}::{}", ev.pezpallet_name(), ev.variant_name());
                            if ev.pezpallet_name() == "Utility" && ev.variant_name() == "BatchCompleted"
                            {
                                has_batch_completed = true;
                            }
                        }
                        if has_batch_completed {
                            println!("\nSUCCESS! wUSDT custody migrated to the multisig.");
                        } else {
                            println!("\nWARNING: expected Utility::BatchCompleted - check events above.");
                        }
                    }
                    Err(e) => println!("DISPATCH ERROR: {}", e),
                }
                break;
            }
            Some(Ok(TxStatus::Error { message })) => {
                println!("TX ERROR: {}", message);
                break;
            }
            Some(Ok(TxStatus::Invalid { message })) => {
                println!("TX INVALID: {}", message);
                break;
            }
            Some(Ok(TxStatus::Dropped { message })) => {
                println!("TX DROPPED: {}", message);
                break;
            }
            Some(Err(e)) => {
                println!("STREAM ERROR: {}", e);
                break;
            }
            None => {
                println!("STREAM ENDED");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
