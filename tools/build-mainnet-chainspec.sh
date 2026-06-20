#!/bin/bash
# =============================================================================
# PEZKUWICHAIN MAINNET CHAIN SPEC BUILDER
# =============================================================================
# Bu script mainnet chain spec'ini doğru şekilde oluşturur:
# 1. Relay chain spec (num_cores: 2 ile)
# 2. Asset Hub (para 1000) genesis_head + validation_code
# 3. People Chain (para 1004) genesis_head + validation_code
# 4. Raw format'a çevirir
# =============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="${1:-$SDK_ROOT/chainspecs}"

PEZKUWI_BIN="$SDK_ROOT/target/release/pezkuwi"
TEYRCHAIN_BIN="$SDK_ROOT/target/release/pezkuwi-teyrchain"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}=== PEZKUWICHAIN MAINNET CHAIN SPEC BUILDER ===${NC}"
echo "SDK Root: $SDK_ROOT"
echo "Output Dir: $OUTPUT_DIR"
echo ""

# Check binaries
if [ ! -f "$PEZKUWI_BIN" ]; then
    echo -e "${RED}ERROR: pezkuwi binary not found at $PEZKUWI_BIN${NC}"
    echo "Run: cargo build --release -p pezkuwi"
    exit 1
fi

if [ ! -f "$TEYRCHAIN_BIN" ]; then
    echo -e "${RED}ERROR: pezkuwi-teyrchain binary not found at $TEYRCHAIN_BIN${NC}"
    echo "Run: cargo build --release -p pezkuwi-teyrchain"
    exit 1
fi

mkdir -p "$OUTPUT_DIR"

# Temp files for large data
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# =============================================================================
# STEP 1: Generate relay chain plain spec
# =============================================================================
echo -e "${YELLOW}[1/6] Generating relay chain plain spec...${NC}"
$PEZKUWI_BIN build-spec \
    --chain pezkuwichain-mainnet \
    --disable-default-bootnode \
    2>/dev/null > "$OUTPUT_DIR/relay-plain.json"
echo -e "${GREEN}  -> relay-plain.json created${NC}"

# =============================================================================
# STEP 2: Generate Asset Hub chain spec
# =============================================================================
echo -e "${YELLOW}[2/6] Generating Asset Hub chain spec...${NC}"
$TEYRCHAIN_BIN build-spec \
    --chain asset-hub-pezkuwichain-genesis \
    --disable-default-bootnode \
    2>/dev/null > "$OUTPUT_DIR/asset-hub-plain.json"
echo -e "${GREEN}  -> asset-hub-plain.json created${NC}"

# =============================================================================
# STEP 3: Generate People Chain chain spec
# =============================================================================
echo -e "${YELLOW}[3/6] Generating People Chain chain spec...${NC}"
$TEYRCHAIN_BIN build-spec \
    --chain people-pezkuwichain-genesis \
    --disable-default-bootnode \
    2>/dev/null > "$OUTPUT_DIR/people-plain.json"
echo -e "${GREEN}  -> people-plain.json created${NC}"

# =============================================================================
# STEP 4: Export genesis data and add to relay spec
# =============================================================================
echo -e "${YELLOW}[4/6] Exporting genesis data and adding paras to relay spec...${NC}"

# Export to temp files (to avoid shell variable size limits)
$TEYRCHAIN_BIN export-genesis-head --chain "$OUTPUT_DIR/asset-hub-plain.json" 2>/dev/null > "$TEMP_DIR/asset-hub-head.txt"
$TEYRCHAIN_BIN export-genesis-wasm --chain "$OUTPUT_DIR/asset-hub-plain.json" 2>/dev/null > "$TEMP_DIR/asset-hub-wasm.txt"
$TEYRCHAIN_BIN export-genesis-head --chain "$OUTPUT_DIR/people-plain.json" 2>/dev/null > "$TEMP_DIR/people-head.txt"
$TEYRCHAIN_BIN export-genesis-wasm --chain "$OUTPUT_DIR/people-plain.json" 2>/dev/null > "$TEMP_DIR/people-wasm.txt"

echo "  Asset Hub head: $(head -c 50 $TEMP_DIR/asset-hub-head.txt)..."
echo "  Asset Hub wasm: $(wc -c < $TEMP_DIR/asset-hub-wasm.txt) bytes"
echo "  People head: $(head -c 50 $TEMP_DIR/people-head.txt)..."
echo "  People wasm: $(wc -c < $TEMP_DIR/people-wasm.txt) bytes"

# Add paras to relay chain spec using Python (reading from files)
python3 << PYEOF
import json

# Read genesis data from files
with open("$TEMP_DIR/asset-hub-head.txt", "r") as f:
    asset_hub_head = f.read().strip()
with open("$TEMP_DIR/asset-hub-wasm.txt", "r") as f:
    asset_hub_wasm = f.read().strip()
with open("$TEMP_DIR/people-head.txt", "r") as f:
    people_head = f.read().strip()
with open("$TEMP_DIR/people-wasm.txt", "r") as f:
    people_wasm = f.read().strip()

# Read relay spec
with open("$OUTPUT_DIR/relay-plain.json", "r") as f:
    spec = json.load(f)

# Navigate to patch
patch = spec.get("genesis", {}).get("runtimeGenesis", {}).get("patch", {})
if not patch:
    patch = spec.get("genesis", {}).get("runtime", {})

# Ensure num_cores is 2
if "configuration" in patch:
    config = patch["configuration"].get("config", {})
    scheduler = config.get("scheduler_params", {})
    scheduler["num_cores"] = 2
    config["scheduler_params"] = scheduler
    patch["configuration"]["config"] = config
    print("  num_cores set to 2")

# Add paras (note: field is "teyrchain" not "para_kind" due to serde rename)
patch["paras"] = {
    "paras": [
        [
            1000,
            {
                "genesis_head": asset_hub_head,
                "validation_code": asset_hub_wasm,
                "teyrchain": True
            }
        ],
        [
            1004,
            {
                "genesis_head": people_head,
                "validation_code": people_wasm,
                "teyrchain": True
            }
        ]
    ]
}
print("  Added paras: 1000 (Asset Hub), 1004 (People Chain)")

# Update spec
if "runtimeGenesis" in spec.get("genesis", {}):
    spec["genesis"]["runtimeGenesis"]["patch"] = patch
else:
    spec["genesis"]["runtime"] = patch

with open("$OUTPUT_DIR/relay-with-paras.json", "w") as f:
    json.dump(spec, f)

print("  -> relay-with-paras.json created")
PYEOF

echo -e "${GREEN}  -> Paras added to relay spec${NC}"

# =============================================================================
# STEP 5: Convert to raw format
# =============================================================================
echo -e "${YELLOW}[5/6] Converting to raw format...${NC}"

echo "  Converting relay chain..."
$PEZKUWI_BIN build-spec \
    --chain "$OUTPUT_DIR/relay-with-paras.json" \
    --raw \
    --disable-default-bootnode \
    2>/dev/null > "$OUTPUT_DIR/relay-raw.json"
echo -e "${GREEN}  -> relay-raw.json created${NC}"

echo "  Converting Asset Hub..."
$TEYRCHAIN_BIN build-spec \
    --chain "$OUTPUT_DIR/asset-hub-plain.json" \
    --raw \
    --disable-default-bootnode \
    2>/dev/null > "$OUTPUT_DIR/asset-hub-raw.json"
echo -e "${GREEN}  -> asset-hub-raw.json created${NC}"

echo "  Converting People Chain..."
$TEYRCHAIN_BIN build-spec \
    --chain "$OUTPUT_DIR/people-plain.json" \
    --raw \
    --disable-default-bootnode \
    2>/dev/null > "$OUTPUT_DIR/people-raw.json"
echo -e "${GREEN}  -> people-raw.json created${NC}"

# =============================================================================
# STEP 6: Verify
# =============================================================================
echo -e "${YELLOW}[6/6] Verifying chain specs...${NC}"

python3 << PYEOF
import json
import sys

def verify_spec(path, name, expected_id):
    try:
        with open(path, 'r') as f:
            spec = json.load(f)

        actual_id = spec.get("id", "")
        if actual_id == expected_id:
            size_mb = len(json.dumps(spec)) / 1024 / 1024
            print(f"  ✓ {name}: id={actual_id} ({size_mb:.1f} MB)")
            return True
        else:
            print(f"  ✗ {name}: expected id={expected_id}, got {actual_id}")
            return False
    except Exception as e:
        print(f"  ✗ {name}: Failed to load - {e}")
        return False

ok = True
ok = verify_spec("$OUTPUT_DIR/relay-raw.json", "Relay Chain", "pezkuwichain_mainnet") and ok
ok = verify_spec("$OUTPUT_DIR/asset-hub-raw.json", "Asset Hub", "asset-hub-pezkuwichain") and ok
ok = verify_spec("$OUTPUT_DIR/people-raw.json", "People Chain", "people-pezkuwichain-genesis") and ok

if not ok:
    sys.exit(1)
PYEOF

# =============================================================================
# DONE
# =============================================================================
echo ""
echo -e "${GREEN}=== CHAIN SPECS CREATED SUCCESSFULLY ===${NC}"
echo ""
echo "Output files:"
ls -lh "$OUTPUT_DIR"/*.json
echo ""
echo "To deploy to VPS:"
echo "  scp $OUTPUT_DIR/relay-raw.json root@VPS_IP:/root/chainspec/"
echo "  scp $OUTPUT_DIR/asset-hub-raw.json root@VPS_IP:/root/chainspec/"
echo "  scp $OUTPUT_DIR/people-raw.json root@VPS_IP:/root/chainspec/"
