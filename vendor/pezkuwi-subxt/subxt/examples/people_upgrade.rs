//! People Chain Runtime Upgrade (Mainnet)
//!
//! Two-step process:
//! 1. RC → XCM → People: System.authorize_upgrade(blake2_256(wasm))
//! 2. People direct: System.apply_authorized_upgrade(wasm)
//!
//! Run:
//!   SUDO_KEY_FILE=/home/myhez/res/genesis/zagros/zagros-wallets.json SUDO_PATH=//zagros//sudo \
//!   RC_RPC="wss://zagros-rpc.pezkuwichain.io" \
//!   PEOPLE_RPC="wss://zagros-people-rpc.pezkuwichain.io" \
//!   WASM_FILE=<people_zagros_runtime.compact.compressed.wasm> \
//!   cargo run --release -p pezkuwi-subxt --example people_upgrade

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use pezkuwi_subxt_signer::SecretUri;
use std::str::FromStr;

const PEOPLE_PARA_ID: u128 = 1004;

/// The sudo signer, from a file and a derivation path.
///
/// This is `ah_upgrade`'s loader. That file was fixed on 2026-09-16 and this one, which does
/// the same job on the sibling chain, was not -- the half-ported pair that keeps costing here.
/// What it was doing wrong:
///
/// It derived from the bare phrase, with no path. The chain's sudo lives at `//zagros//sudo`,
/// and the bare phrase is a *different account* -- a stranger with no balance. The chain then
/// reports an inability to pay fees, which reads like the target is wrong when the signer is.
///
/// It also fell back to `/home/mamostehp/res/test_seeds.json`, a path on a machine that is not
/// this one, so the fallback could only ever fail -- while still being a hardcoded location to
/// read a secret from.
///
/// And it took the phrase from the environment, where it lands in the process list. A file
/// read keeps it out.
fn load_sudo_keypair() -> Keypair {
	let path =
		std::env::var("SUDO_KEY_FILE").unwrap_or_else(|_| "/home/myhez/res/sudo.json".to_string());
	let content =
		std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {}", path, e));

	let mnemonic_str = if path.ends_with(".json") {
		let json: serde_json::Value =
			serde_json::from_str(&content).expect("sudo key file is not valid JSON");
		// `mnemonic` is mainnet's shape; `_master_phrase` is the generated wallet set's, where
		// the root key is a derivation rather than the phrase itself.
		json["mnemonic"]
			.as_str()
			.or_else(|| json["_master_phrase"].as_str())
			.unwrap_or_else(|| {
				panic!("{} has neither a `mnemonic` nor a `_master_phrase` field", path)
			})
			.to_string()
	} else {
		// Markdown: the line reads ``- **Mnemonic:** `word word ...` ``
		content
			.lines()
			.find(|l| l.contains("Mnemonic:"))
			.and_then(|l| l.split('`').nth(1))
			.unwrap_or_else(|| panic!("{} has no `**Mnemonic:** \\`...\\`` line", path))
			.trim()
			.to_string()
	};

	let path_suffix = std::env::var("SUDO_PATH").unwrap_or_default();
	let signer = if path_suffix.is_empty() {
		let mnemonic =
			Mnemonic::from_str(&mnemonic_str).expect("invalid mnemonic in sudo key file");
		Keypair::from_phrase(&mnemonic, None).expect("cannot derive keypair from mnemonic")
	} else {
		let uri = SecretUri::from_str(&format!("{mnemonic_str}{path_suffix}"))
			.expect("phrase + SUDO_PATH is not a valid secret uri");
		Keypair::from_uri(&uri).expect("cannot derive keypair from phrase and path")
	};
	println!("  [sudo] Loaded from {} (path {:?})", path, path_suffix);
	println!("  [sudo] {}", signer.public_key().to_account_id());
	signer
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("╔══════════════════════════════════════════╗");
	println!("║  PEOPLE CHAIN RUNTIME UPGRADE            ║");
	println!("╚══════════════════════════════════════════╝\n");

	let rc_url = std::env::var("RC_RPC").unwrap_or_else(|_| "ws://127.0.0.1:9944".to_string());
	let people_url =
		std::env::var("PEOPLE_RPC").unwrap_or_else(|_| "ws://127.0.0.1:41944".to_string());
	let wasm_path = std::env::var("WASM_FILE").expect("WASM_FILE environment variable required");

	let sudo_keypair = load_sudo_keypair();
	println!("  Sudo: {}", sudo_keypair.public_key().to_account_id());

	// Load WASM
	let wasm_data = std::fs::read(&wasm_path)?;
	println!("  WASM: {} ({:.2} MB)", wasm_path, wasm_data.len() as f64 / 1_048_576.0);

	// Blake2-256 hash of WASM
	let code_hash = pezsp_crypto_hashing::blake2_256(&wasm_data);
	println!("  Code hash: 0x{}", hex::encode(code_hash));

	// Connect to RC
	let rc_api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&rc_url).await?;
	println!("  RC connected: {} (spec {})", rc_url, rc_api.runtime_version().spec_version);

	// Connect to People Chain
	let people_api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&people_url).await?;
	let old_spec = people_api.runtime_version().spec_version;
	println!("  People connected: {} (spec {})\n", people_url, old_spec);

	// ═══════════════════════════════════════════
	// STEP 1: Authorize upgrade via XCM from RC
	// ═══════════════════════════════════════════
	println!("=== STEP 1: Authorize upgrade (RC → XCM → People) ===");

	// `authorize_upgrade`, not `authorize_upgrade_without_checks`. The two differ by one flag,
	// and the flag turns on the check that the blob's `spec_name` belongs to this chain and its
	// `spec_version` is higher than the running one. This network is twins -- Zagros and
	// Pezkuwichain build near-identical artefacts from one tree -- so authorising the wrong
	// twin's blob is a plausible slip on a tired evening, not a theoretical one. Skipping the
	// check is upstream's escape hatch for a chain that has to be recovered; using it as the
	// default in every upgrade tool removes the guard for the ordinary case too.
	// Encode System::authorize_upgrade(code_hash)
	// System pallet index = 0, call_index = 9
	let mut encoded_call = Vec::with_capacity(34);
	encoded_call.push(0x00); // System pallet
	encoded_call.push(0x09); // authorize_upgrade (9)
	encoded_call.extend_from_slice(&code_hash);
	println!("  Encoded call: {} bytes", encoded_call.len());

	let dest = Value::unnamed_variant(
		"V3",
		vec![Value::named_composite([
			("parents", Value::u128(0)),
			(
				"interior",
				Value::unnamed_variant(
					"X1",
					vec![Value::unnamed_variant("Teyrchain", vec![Value::u128(PEOPLE_PARA_ID)])],
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
							("ref_time", Value::u128(5_000_000_000u128)),
							("proof_size", Value::u128(500_000u128)),
						]),
					),
					("call", Value::from_bytes(&encoded_call)),
				],
			),
		])],
	);

	let xcm_send = pezkuwi_subxt::dynamic::tx("XcmPallet", "send", vec![dest, message]);
	let sudo_tx = pezkuwi_subxt::dynamic::tx(
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

	let progress = rc_api.tx().sign_and_submit_then_watch_default(&sudo_tx, &sudo_keypair).await?;
	let events = progress.wait_for_finalized_success().await?;

	let mut sent = false;
	for event in events.iter() {
		let event = event?;
		if event.pallet_name() == "XcmPallet" && event.variant_name() == "Sent" {
			sent = true;
		}
		if event.pallet_name() == "Sudo" || event.pallet_name() == "XcmPallet" {
			println!("  {}::{}", event.pallet_name(), event.variant_name());
		}
	}
	if !sent {
		println!("  ERROR: No XcmPallet::Sent event! Aborting.");
		return Ok(());
	}
	println!("  XCM authorize_upgrade sent!\n");

	// Wait for People Chain to process the XCM — poll AuthorizedUpgrade storage
	println!("Waiting for People Chain to process XCM authorize_upgrade...");
	let mut authorized = false;
	for attempt in 1..=30 {
		tokio::time::sleep(std::time::Duration::from_secs(6)).await;

		let people_check = OnlineClient::<PezkuwiConfig>::from_insecure_url(&people_url).await?;
		let block = people_check.blocks().at_latest().await?;
		let block_num = block.number();

		// Check System::AuthorizedUpgrade storage via raw key
		let auth_key = pezsp_crypto_hashing::twox_128(b"System")
			.iter()
			.chain(pezsp_crypto_hashing::twox_128(b"AuthorizedUpgrade").iter())
			.copied()
			.collect::<Vec<u8>>();
		let result = people_check.storage().at_latest().await?.fetch_raw(auth_key).await;
		match result {
			Ok(data) if !data.is_empty() => {
				println!(
					"  AuthorizedUpgrade found on People at block {} (attempt {})!",
					block_num, attempt
				);
				authorized = true;
				break;
			},
			_ => {}, // NoValueFound or empty — not yet set, continue polling
		}
		println!(
			"  Attempt {}/30: People block {} — AuthorizedUpgrade not yet set...",
			attempt, block_num
		);
	}

	if !authorized {
		println!("  ERROR: AuthorizedUpgrade not set after 3 minutes. Aborting.");
		return Ok(());
	}

	// ═══════════════════════════════════════════
	// STEP 1.5: Fund sudo account on People via XCM (if needed)
	// ═══════════════════════════════════════════
	println!("\n=== STEP 1.5: Fund sudo account on People Chain ===");
	let sudo_account_id = sudo_keypair.public_key().to_account_id();
	let account_bytes: [u8; 32] = *sudo_account_id.as_ref();

	// Check existing balance first
	let balance_key = {
		let mut key = Vec::new();
		key.extend_from_slice(&pezsp_crypto_hashing::twox_128(b"System"));
		key.extend_from_slice(&pezsp_crypto_hashing::twox_128(b"Account"));
		// Blake2_128Concat hasher for account
		let hash = pezsp_crypto_hashing::blake2_128(&account_bytes);
		key.extend_from_slice(&hash);
		key.extend_from_slice(&account_bytes);
		key
	};
	let people_storage = people_api.storage().at_latest().await?;
	let has_balance = match people_storage.fetch_raw(balance_key).await {
		Ok(data) => !data.is_empty(),
		Err(_) => false, // NoValueFound — account doesn't exist
	};

	if has_balance {
		println!("  Sudo account already has funds on People Chain — skipping funding");
	} else {
		println!("  Sudo account has no funds — funding via XCM...");

		// Encode Balances::force_set_balance(who, new_free)
		// Balances pallet = 10, call_index = 8
		let mut fund_call: Vec<u8> = Vec::new();
		fund_call.push(10u8); // Balances pallet
		fund_call.push(8u8); // force_set_balance
		fund_call.push(0u8); // MultiAddress::Id variant
		fund_call.extend_from_slice(&account_bytes);
		// 10,000 HEZ = 10_000 * 10^12 (12 decimals, NOT 18)
		// Generous amount to cover apply_authorized_upgrade fee (1.5MB extrinsic)
		let amount: u128 = 10_000_000_000_000_000u128; // 10,000 HEZ
		let amount_bytes = amount.to_le_bytes();
		let significant = amount_bytes.iter().rposition(|&b| b != 0).map(|i| i + 1).unwrap_or(1);
		let byte_len = significant.max(4);
		fund_call.push(((byte_len as u8 - 4) << 2) | 0b11);
		fund_call.extend_from_slice(&amount_bytes[..byte_len]);

		let fund_dest = Value::unnamed_variant(
			"V3",
			vec![Value::named_composite([
				("parents", Value::u128(0)),
				(
					"interior",
					Value::unnamed_variant(
						"X1",
						vec![Value::unnamed_variant(
							"Teyrchain",
							vec![Value::u128(PEOPLE_PARA_ID)],
						)],
					),
				),
			])],
		);

		let fund_msg = Value::unnamed_variant(
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
								("ref_time", Value::u128(5_000_000_000u128)),
								("proof_size", Value::u128(500_000u128)),
							]),
						),
						("call", Value::from_bytes(&fund_call)),
					],
				),
			])],
		);

		let fund_xcm = pezkuwi_subxt::dynamic::tx("XcmPallet", "send", vec![fund_dest, fund_msg]);
		let fund_sudo = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![fund_xcm.into_value()]);

		let progress = rc_api
			.tx()
			.sign_and_submit_then_watch_default(&fund_sudo, &sudo_keypair)
			.await?;
		let events = progress.wait_for_finalized_success().await?;
		let fund_sent = events
			.iter()
			.flatten()
			.any(|e| e.pallet_name() == "XcmPallet" && e.variant_name() == "Sent");
		if fund_sent {
			println!("  [OK] Force set balance XCM sent");
		} else {
			println!("  [WARN] No XcmPallet::Sent event for funding");
		}

		println!("  Waiting 18s for DMP processing...");
		tokio::time::sleep(std::time::Duration::from_secs(18)).await;
	}

	// ═══════════════════════════════════════════
	// STEP 2: Enact upgrade on People directly
	// ═══════════════════════════════════════════
	println!("\n=== STEP 2: Apply authorized upgrade on People Chain ===");
	println!("  Submitting {} bytes WASM...", wasm_data.len());

	let enact_call = pezkuwi_subxt::dynamic::tx(
		"System",
		"apply_authorized_upgrade",
		vec![Value::from_bytes(&wasm_data)],
	);

	let progress = people_api
		.tx()
		.sign_and_submit_then_watch_default(&enact_call, &sudo_keypair)
		.await?;
	let events = progress.wait_for_finalized_success().await?;

	let mut code_updated = false;
	for event in events.iter() {
		let event = event?;
		println!("  {}::{}", event.pallet_name(), event.variant_name());
		if event.pallet_name() == "System" && event.variant_name() == "CodeUpdated" {
			code_updated = true;
		}
	}

	if code_updated {
		println!("\n  UPGRADE SUCCESS!");
	} else {
		println!("\n  WARNING: No CodeUpdated event!");
	}

	// ═══════════════════════════════════════════
	// STEP 3: Verify
	// ═══════════════════════════════════════════
	// `ValidationFunctionStored` means the teyrchain accepted the code, not that it is running
	// it: enactment waits for the relay, which takes a couple of minutes. A single sleep is a
	// guess about that, and on 2026-09-18 the guess was twelve seconds against a wait of nearly
	// three -- so this printed "did not increase" and then "UPGRADE COMPLETE" underneath it,
	// while the chain had in fact upgraded. A banner that is right by the time anybody reads it
	// is worse than a red one, because nobody goes back to check.
	//
	// So poll, like `ah_upgrade` does, and make the exit code carry the answer.
	let mut new_spec = old_spec;
	for attempt in 1..=15 {
		tokio::time::sleep(std::time::Duration::from_secs(12)).await;
		// A fresh client each time. The runtime version is cached at connect, so asking the
		// same handle again would be the check confirming itself.
		let probe = OnlineClient::<PezkuwiConfig>::from_insecure_url(&people_url).await?;
		new_spec = probe.runtime_version().spec_version;
		if new_spec > old_spec {
			println!(
				"  People spec_version: {old_spec} → {new_spec} — VERIFIED (attempt {attempt})"
			);
			println!("\n╔══════════════════════════════════════════╗");
			println!("║  PEOPLE CHAIN UPGRADE COMPLETE           ║");
			println!("╚══════════════════════════════════════════╝");
			return Ok(());
		}
		println!("    Attempt {attempt}/15: spec still {new_spec} — waiting...");
	}

	Err(format!(
		"People never left spec_version {old_spec} after three minutes. The code was stored, so \
		 look at relay enactment rather than at the submission -- and do not re-run the upgrade \
		 until you know which, because a second authorize would queue a second enactment."
	)
	.into())
}
