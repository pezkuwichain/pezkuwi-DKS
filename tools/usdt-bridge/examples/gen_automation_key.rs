use anyhow::Result;

fn main() -> Result<()> {
    let (pair, phrase, _) = sp_core::sr25519::Pair::generate_with_phrase(None);
    use sp_core::Pair as _;
    let public = pair.public();
    let account = sp_core::crypto::AccountId32::from(public.0);
    let ss58 = sp_core::crypto::Ss58Codec::to_ss58check_with_version(
        &account,
        sp_core::crypto::Ss58AddressFormat::custom(42),
    );

    let out = serde_json::json!({
        "warning": "CONTAINS PRIVATE SEED PHRASE - KEEP SECURE - NOT IN GIT - NOT IN LOGS",
        "role": "usdt-bridge relayer automation key - LIMITED delegated allowance only via Assets.approve_transfer, never asset owner/admin",
        "ss58_address_pezkuwi": ss58,
        "scheme": "sr25519",
        "mnemonic": phrase,
    });
    std::fs::write("bridge_automation_key.json", serde_json::to_string_pretty(&out)?)?;

    println!("Generated automation key.");
    println!("Address (never the mnemonic): {}", ss58);
    println!("Saved to bridge_automation_key.json");

    Ok(())
}
