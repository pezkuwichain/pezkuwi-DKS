//! Chain Spec Tool - Add teyrchains to relay chain spec
//!
//! This tool modifies plain chain specs to include teyrchain genesis data.
//! It's a standalone extraction of the zombienet-sdk logic for manual use.

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "chain-spec-tool")]
#[command(about = "CLI tool for modifying Pezkuwi chain specs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a teyrchain to a relay chain spec
    AddTeyrchain {
        /// Path to the plain chain spec JSON file
        #[arg(long, short = 'c')]
        chain_spec: PathBuf,

        /// Path to the teyrchain genesis state hex file
        #[arg(long, short = 's')]
        state: PathBuf,

        /// Path to the teyrchain validation code (wasm) hex file
        #[arg(long, short = 'w')]
        wasm: PathBuf,

        /// Teyrchain ID (e.g., 1000 for Asset Hub)
        #[arg(long, short = 'i')]
        id: u32,

        /// Output path for the modified chain spec
        #[arg(long, short = 'o')]
        output: PathBuf,

        /// Register as teyrchain (true) or parathread (false)
        #[arg(long, default_value = "true")]
        as_teyrchain: bool,
    },

    /// Show info about a chain spec
    Info {
        /// Path to the chain spec JSON file
        #[arg(long, short = 'c')]
        chain_spec: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::AddTeyrchain {
            chain_spec,
            state,
            wasm,
            id,
            output,
            as_teyrchain,
        } => {
            add_teyrchain_to_spec(&chain_spec, &state, &wasm, id, &output, as_teyrchain)?;
        }
        Commands::Info { chain_spec } => {
            show_chain_spec_info(&chain_spec)?;
        }
    }

    Ok(())
}

/// Recursively fix scientific notation in JSON values
/// Converts floats like 2e+19 back to integers
fn fix_scientific_notation(value: &mut Value) {
    match value {
        Value::Number(n) => {
            // If it's a float that can be represented as integer, convert it
            if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 && f >= 0.0 && f <= u128::MAX as f64 {
                    // Convert to u128 string, then parse back to Number
                    let int_val = f as u128;
                    if let Ok(new_num) = serde_json::Number::from_str(&int_val.to_string()) {
                        *n = new_num;
                    }
                }
            }
        }
        Value::Array(arr) => {
            for item in arr {
                fix_scientific_notation(item);
            }
        }
        Value::Object(map) => {
            for (_, v) in map {
                fix_scientific_notation(v);
            }
        }
        _ => {}
    }
}

/// Get the runtime config pointer from a chain spec JSON
fn get_runtime_config_pointer(chain_spec_json: &Value) -> Result<String> {
    let pointers = [
        "/genesis/runtimeGenesis/config",
        "/genesis/runtimeGenesis/patch",
        "/genesis/runtimeGenesisConfigPatch",
        "/genesis/runtime/runtime_genesis_config",
        "/genesis/runtime",
    ];

    for pointer in pointers {
        if chain_spec_json.pointer(pointer).is_some() {
            return Ok(pointer.to_string());
        }
    }

    Err(anyhow!("Cannot find the runtime config pointer in chain spec"))
}

/// Add a teyrchain to a relay chain spec
fn add_teyrchain_to_spec(
    chain_spec_path: &PathBuf,
    state_path: &PathBuf,
    wasm_path: &PathBuf,
    para_id: u32,
    output_path: &PathBuf,
    as_teyrchain: bool,
) -> Result<()> {
    println!("Reading chain spec from: {}", chain_spec_path.display());

    // Read chain spec
    let content = fs::read_to_string(chain_spec_path)
        .with_context(|| format!("Failed to read chain spec: {}", chain_spec_path.display()))?;

    let mut chain_spec_json: Value = serde_json::from_str(&content)
        .with_context(|| "Failed to parse chain spec as JSON")?;

    // Check if it's raw format
    if chain_spec_json.pointer("/genesis/raw/top").is_some() {
        return Err(anyhow!(
            "Chain spec is in RAW format. This tool only works with PLAIN format.\n\
             Generate a plain chain spec first, then convert to raw after adding teyrchains."
        ));
    }

    // Get runtime config pointer
    let runtime_ptr = get_runtime_config_pointer(&chain_spec_json)?;
    println!("Found runtime config at: {}", runtime_ptr);

    // Read genesis state and wasm
    let genesis_head = fs::read_to_string(state_path)
        .with_context(|| format!("Failed to read genesis state: {}", state_path.display()))?;

    let validation_code = fs::read_to_string(wasm_path)
        .with_context(|| format!("Failed to read validation code: {}", wasm_path.display()))?;

    println!("Genesis state size: {} bytes", genesis_head.trim().len());
    println!("Validation code size: {} bytes", validation_code.trim().len());

    // Add teyrchain to genesis
    add_parachain_to_genesis(
        &runtime_ptr,
        &mut chain_spec_json,
        para_id,
        genesis_head.trim(),
        validation_code.trim(),
        as_teyrchain,
    )?;

    println!("Added teyrchain {} to genesis", para_id);

    // Fix scientific notation in JSON (Rust parser doesn't accept 2e+19 format)
    // Convert all floats that are actually integers back to integers
    fix_scientific_notation(&mut chain_spec_json);

    // Write output
    let output_content = serde_json::to_string_pretty(&chain_spec_json)
        .with_context(|| "Failed to serialize chain spec")?;

    fs::write(output_path, output_content)
        .with_context(|| format!("Failed to write output: {}", output_path.display()))?;

    println!("Wrote modified chain spec to: {}", output_path.display());
    println!("\nNext step: Convert to raw format with:");
    println!("  ./target/release/pezkuwi build-spec --chain {} --raw > <raw-output.json>", output_path.display());

    Ok(())
}

/// Add a parachain to the genesis config (extracted from zombienet-sdk)
fn add_parachain_to_genesis(
    runtime_config_ptr: &str,
    chain_spec_json: &mut Value,
    para_id: u32,
    genesis_head: &str,
    validation_code: &str,
    as_teyrchain: bool,
) -> Result<()> {
    let val = chain_spec_json
        .pointer_mut(runtime_config_ptr)
        .ok_or_else(|| anyhow!("Runtime config pointer not found: {}", runtime_config_ptr))?;

    // Determine paras pointer
    let paras_pointer = if val.get("paras").is_some() {
        "/paras/paras"
    } else if val.get("parachainsParas").is_some() {
        // For retro-compatibility with substrate pre Polkadot 0.9.5
        "/parachainsParas/paras"
    } else {
        // The config may not contain paras. Since chainspec allows RuntimeGenesisConfig patch we can inject it.
        val["paras"] = json!({ "paras": [] });
        "/paras/paras"
    };

    let paras = val
        .pointer_mut(paras_pointer)
        .ok_or_else(|| anyhow!("Paras pointer not found: {}", paras_pointer))?;

    let paras_vec = paras
        .as_array_mut()
        .ok_or_else(|| anyhow!("Paras should be an array"))?;

    // Check if para_id already exists
    for existing in paras_vec.iter() {
        if let Some(existing_id) = existing.get(0).and_then(|v| v.as_u64()) {
            if existing_id == para_id as u64 {
                return Err(anyhow!("Teyrchain {} already exists in genesis", para_id));
            }
        }
    }

    // Use object format for ParaGenesisArgs as expected by runtime
    // Note: field is renamed to "teyrchain" via #[serde(rename = "teyrchain")] in runtime
    // Boolean value: true = Teyrchain, false = Parathread
    paras_vec.push(json!([
        para_id,
        {
            "genesis_head": genesis_head,
            "validation_code": validation_code,
            "teyrchain": as_teyrchain
        }
    ]));

    Ok(())
}

/// Show information about a chain spec
fn show_chain_spec_info(chain_spec_path: &PathBuf) -> Result<()> {
    let content = fs::read_to_string(chain_spec_path)
        .with_context(|| format!("Failed to read chain spec: {}", chain_spec_path.display()))?;

    let chain_spec_json: Value = serde_json::from_str(&content)
        .with_context(|| "Failed to parse chain spec as JSON")?;

    // Basic info
    println!("Chain Spec Information");
    println!("======================");

    if let Some(name) = chain_spec_json.get("name").and_then(|v| v.as_str()) {
        println!("Name: {}", name);
    }

    if let Some(id) = chain_spec_json.get("id").and_then(|v| v.as_str()) {
        println!("ID: {}", id);
    }

    if let Some(chain_type) = chain_spec_json.get("chainType").and_then(|v| v.as_str()) {
        println!("Chain Type: {}", chain_type);
    }

    // Check format
    let is_raw = chain_spec_json.pointer("/genesis/raw/top").is_some();
    println!("Format: {}", if is_raw { "RAW" } else { "PLAIN" });

    // Check for paras
    if !is_raw {
        if let Ok(runtime_ptr) = get_runtime_config_pointer(&chain_spec_json) {
            println!("Runtime Config: {}", runtime_ptr);

            if let Some(val) = chain_spec_json.pointer(&runtime_ptr) {
                let paras_pointer = if val.get("paras").is_some() {
                    "/paras/paras"
                } else if val.get("parachainsParas").is_some() {
                    "/parachainsParas/paras"
                } else {
                    ""
                };

                if !paras_pointer.is_empty() {
                    let full_ptr = format!("{}{}", runtime_ptr, paras_pointer);
                    if let Some(paras) = chain_spec_json.pointer(&full_ptr) {
                        if let Some(paras_arr) = paras.as_array() {
                            println!("\nRegistered Teyrchains: {}", paras_arr.len());
                            for para in paras_arr {
                                if let Some(id) = para.get(0).and_then(|v| v.as_u64()) {
                                    let is_teyrchain = para
                                        .get(1)
                                        .and_then(|v| v.get("teyrchain"))
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(true);
                                    println!(
                                        "  - ID: {} ({})",
                                        id,
                                        if is_teyrchain { "teyrchain" } else { "parathread" }
                                    );
                                }
                            }
                        }
                    }
                } else {
                    println!("\nNo teyrchains registered in genesis");
                }
            }
        }
    }

    Ok(())
}
