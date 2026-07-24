//! Grant Xezinedar (Validator_01) and Berdevk (Validator_02) tiki via XCM Transact,
//! following the exact proven pattern from vendor/pezkuwi-subxt/subxt/examples/grant_noter_tiki.rs
//! (People Chain has no local Sudo pallet - Root must arrive via XCM from the relay chain).

#![allow(missing_docs, dead_code)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const PEOPLE_CHAIN_PARA_ID: u32 = 1004;

// People Chain pallet/call indices - independently verified against live metadata this session.
const UTILITY_PALLET: u8 = 40;
const TIKI_PALLET: u8 = 61;
const GRANT_TIKI_CALL: u8 = 0;
const BATCH_ALL_CALL: u8 = 2;

// Tiki enum values - verified against live metadata (Tiki::pezpallet::Tiki declaration order).
const TIKI_XEZINEDAR: u8 = 10;
const TIKI_BERDEVK: u8 = 16;

fn encode_compact(value: usize) -> Vec<u8> {
    if value < 64 {
        vec![(value as u8) << 2]
    } else if value < 16384 {
        let v = ((value as u16) << 2) | 0x01;
        v.to_le_bytes().to_vec()
    } else {
        panic!("Value too large for compact encoding: {}", value);
    }
}

fn encode_grant_tiki(account_id: &[u8; 32], tiki: u8) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(36);
    encoded.push(TIKI_PALLET);
    encoded.push(GRANT_TIKI_CALL);
    encoded.push(0x00); // MultiAddress::Id variant
    encoded.extend_from_slice(account_id);
    encoded.push(tiki);
    encoded
}

fn encode_batch_all(calls: Vec<Vec<u8>>) -> Vec<u8> {
    let mut encoded = vec![UTILITY_PALLET, BATCH_ALL_CALL];
    encoded.extend(encode_compact(calls.len()));
    for call in calls {
        encoded.extend(call);
    }
    encoded
}

fn build_xcm_values(encoded_call: &[u8]) -> (Value, Value) {
    let dest = Value::unnamed_variant(
        "V3",
        vec![Value::named_composite([
            ("parents", Value::u128(0)),
            (
                "interior",
                Value::unnamed_variant(
                    "X1",
                    vec![Value::unnamed_variant(
                        "Teyrchain",
                        vec![Value::u128(PEOPLE_CHAIN_PARA_ID as u128)],
                    )],
                ),
            ),
        ])],
    );

    let message = Value::unnamed_variant(
        "V3",
        vec![Value::unnamed_composite(vec![
            Value::named_variant(
                "UnpaidExecution",
                [
                    ("weight_limit", Value::unnamed_variant("Unlimited", vec![])),
                    ("check_origin", Value::unnamed_variant("None", vec![])),
                ],
            ),
            Value::named_variant(
                "Transact",
                [
                    ("origin_kind", Value::unnamed_variant("Superuser", vec![])),
                    (
                        "require_weight_at_most",
                        Value::named_composite([
                            ("ref_time", Value::u128(50_000_000_000u128)),
                            ("proof_size", Value::u128(5_000_000u128)),
                        ]),
                    ),
                    ("call", Value::from_bytes(encoded_call)),
                ],
            ),
        ])],
    );

    (dest, message)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== GRANT XEZINEDAR + BERDEVK TIKI ===\n");

    let relay_url =
        std::env::var("RPC_URL").unwrap_or_else(|_| "ws://127.0.0.1:9944".to_string());

    let validator_01: AccountId32 = "5GipBJs2uNWTCazyZQ2vG3DEqLz4tXNmNZtBAT1Mtm1orZ5i".parse()?;
    let validator_02: AccountId32 = "5HWFZbhkZuTUySXu6ZXYKrTHBnWXHvWRKLozE22zhnwXGGxk".parse()?;

    let call_xezinedar = encode_grant_tiki(&validator_01.0, TIKI_XEZINEDAR);
    let call_berdevk = encode_grant_tiki(&validator_02.0, TIKI_BERDEVK);
    println!("grant_tiki(Validator_01, Xezinedar): {} bytes", call_xezinedar.len());
    println!("grant_tiki(Validator_02, Berdevk): {} bytes", call_berdevk.len());

    let batch_call = encode_batch_all(vec![call_xezinedar, call_berdevk]);
    println!("Batch call: {} bytes\n", batch_call.len());

    let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&relay_url).await?;
    println!("Connected to {}! specVersion: {}\n", relay_url, api.runtime_version().spec_version);

    let seed_content = std::fs::read_to_string("sudo.json")?;
    let seed_json: serde_json::Value = serde_json::from_str(&seed_content)?;
    let mnemonic_str = seed_json["mnemonic"].as_str().expect("mnemonic field present");
    let mnemonic = Mnemonic::from_str(mnemonic_str)?;
    let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
    println!("Sudo: {}\n", sudo_keypair.public_key().to_account_id());

    let (dest, message) = build_xcm_values(&batch_call);
    let xcm_send = pezkuwi_subxt::dynamic::tx("XcmPallet", "send", vec![dest, message]);

    let sudo_call = pezkuwi_subxt::dynamic::tx(
        "Sudo",
        "sudo_unchecked_weight",
        vec![
            xcm_send.into_value(),
            Value::named_composite([
                ("ref_time", Value::u128(1u128)),
                ("proof_size", Value::u128(1u128)),
            ]),
        ],
    );

    println!("Submitting sudo(xcm.send(batch_all(grant_tiki x2)))...");

    use pezkuwi_subxt::tx::TxStatus;
    let tx_progress =
        api.tx().sign_and_submit_then_watch_default(&sudo_call, &sudo_keypair).await?;

    println!("TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

    let mut progress = tx_progress;
    loop {
        match progress.next().await {
            Some(Ok(TxStatus::InBestBlock(details))) => {
                match details.wait_for_success().await {
                    Ok(events) => {
                        let mut has_sudid = false;
                        let mut has_sent = false;
                        for ev in events.iter().flatten() {
                            println!("  event: {}::{}", ev.pallet_name(), ev.variant_name());
                            if ev.pallet_name() == "Sudo" && ev.variant_name() == "Sudid" {
                                has_sudid = true;
                            }
                            if ev.pallet_name() == "XcmPallet" && ev.variant_name() == "Sent" {
                                has_sent = true;
                            }
                        }
                        if has_sudid && has_sent {
                            println!("\nSUCCESS! XCM batch sent to People Chain.");
                        } else {
                            println!("\nWARNING: expected Sudo::Sudid + XcmPallet::Sent");
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
