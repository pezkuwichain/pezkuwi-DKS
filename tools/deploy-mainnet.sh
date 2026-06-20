#!/bin/bash
# =============================================================================
# PEZKUWICHAIN MAINNET DEPLOYMENT SCRIPT
# =============================================================================
# Bu script yeni mainnet'i sıfırdan deploy eder:
#   1. Tüm node'ları durdurur (silmez)
#   2. Eski chain dizinini rename eder (veri korunur)
#   3. Yeni binary + chain spec dağıtır
#   4. Session key'leri inject eder
#   5. Node'ları sıralı başlatır
#
# Kullanım:
#   bash tools/deploy-mainnet.sh [faz]
#
# Fazlar:
#   check     - Ön kontroller (binary, spec, wallet dosyaları var mı)
#   stop      - Tüm node'ları durdur
#   archive   - Eski chain dizinlerini rename et
#   deploy    - Binary + chain spec dağıt
#   inject    - Session key'leri inject et
#   start     - Node'ları sıralı başlat
#   verify    - Sağlık kontrolü
#   all       - Hepsini sırayla çalıştır
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_ROOT="$(dirname "$SCRIPT_DIR")"

# Dosya yolları
VPS_JSON="/home/mamostehp/claude/vps.json"
WALLETS_JSON="/home/mamostehp/res/MAINNET_WALLETS_20260128_235407.json"
PEZKUWI_BIN="$SDK_ROOT/target/release/pezkuwi"
TEYRCHAIN_BIN="$SDK_ROOT/target/release/pezkuwi-teyrchain"
CHAINSPEC_DIR="$SDK_ROOT/chainspecs"

# Uzak makine yolları
REMOTE_BIN_DIR="/usr/local/bin"
REMOTE_SPEC_DIR="/root/chainspec"
REMOTE_DATA_DIR="/root/.local/share/pezkuwi/chains"
OLD_CHAIN_ID="pezkuwichain_mainnet"
ARCHIVE_SUFFIX="zagros_archive_$(date +%Y%m%d)"

# Renkler
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# ===== VPS Bilgileri (vps.json'dan) =====

# Bootnode IP
BOOTNODE_IP="217.77.6.126"

# Tüm VPS IP'leri (validator çalıştıranlar)
declare -A VPS_VALIDATORS
VPS_VALIDATORS=(
    ["62.146.235.186"]="5 6 7 21"
    ["217.77.6.126"]="1 2 3 4"
    ["217.77.15.51"]="14"
    ["161.97.183.44"]="13"
    ["161.97.185.100"]="12"
    ["109.123.229.159"]="16"
    ["161.97.116.241"]="17"
    ["46.250.241.121"]="18"
    ["164.68.121.181"]="19"
    ["158.220.93.23"]="20"
    ["207.180.194.103"]="11"
    ["167.86.70.241"]="10"
    ["167.86.108.190"]="9"
    ["207.180.233.147"]="8"
    ["178.18.252.120"]="15"
)

# Collator VPS'leri
declare -A VPS_COLLATORS
VPS_COLLATORS=(
    ["217.77.6.126"]="azad erin"
    ["173.249.57.228"]="beritan firaz"
)

ALL_VALIDATOR_IPS=(${!VPS_VALIDATORS[@]})
ALL_COLLATOR_IPS=(${!VPS_COLLATORS[@]})

# Tüm unique IP'ler
ALL_IPS=($(echo "${ALL_VALIDATOR_IPS[@]} ${ALL_COLLATOR_IPS[@]}" | tr ' ' '\n' | sort -u))

# ===== Yardımcı Fonksiyonlar =====

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${CYAN}[STEP]${NC} $1"; }

ssh_cmd() {
    local ip=$1
    shift
    ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no "root@$ip" "$@"
}

scp_file() {
    local src=$1
    local ip=$2
    local dst=$3
    scp -o ConnectTimeout=10 -o StrictHostKeyChecking=no "$src" "root@$ip:$dst"
}

# ===== FAZ: CHECK =====
do_check() {
    log_step "=== ÖN KONTROLLER ==="

    local ok=true

    # Binary kontrolleri
    if [ -f "$PEZKUWI_BIN" ]; then
        log_info "pezkuwi binary: OK ($(du -h "$PEZKUWI_BIN" | cut -f1))"
    else
        log_error "pezkuwi binary bulunamadı: $PEZKUWI_BIN"
        ok=false
    fi

    if [ -f "$TEYRCHAIN_BIN" ]; then
        log_info "pezkuwi-teyrchain binary: OK ($(du -h "$TEYRCHAIN_BIN" | cut -f1))"
    else
        log_error "pezkuwi-teyrchain binary bulunamadı: $TEYRCHAIN_BIN"
        ok=false
    fi

    # Chain spec kontrolleri
    for spec in relay-raw.json asset-hub-raw.json people-raw.json; do
        if [ -f "$CHAINSPEC_DIR/$spec" ]; then
            log_info "Chain spec $spec: OK ($(du -h "$CHAINSPEC_DIR/$spec" | cut -f1))"
        else
            log_error "Chain spec bulunamadı: $CHAINSPEC_DIR/$spec"
            ok=false
        fi
    done

    # Wallet dosyası
    if [ -f "$WALLETS_JSON" ]; then
        local wallet_count
        wallet_count=$(python3 -c "import json; print(len(json.load(open('$WALLETS_JSON'))['wallets']))")
        log_info "Wallet dosyası: OK ($wallet_count wallet)"
    else
        log_error "Wallet dosyası bulunamadı: $WALLETS_JSON"
        ok=false
    fi

    # VPS erişim kontrolü
    log_step "VPS erişim kontrolü (SSH)..."
    for ip in "${ALL_IPS[@]}"; do
        if ssh_cmd "$ip" "echo ok" >/dev/null 2>&1; then
            log_info "  $ip: erişilebilir"
        else
            log_error "  $ip: ERİŞİLEMİYOR"
            ok=false
        fi
    done

    if $ok; then
        log_info "Tüm kontroller başarılı!"
    else
        log_error "Bazı kontroller başarısız. Düzelttikten sonra tekrar deneyin."
        return 1
    fi
}

# ===== FAZ: STOP =====
do_stop() {
    log_step "=== TÜM NODE'LARI DURDUR ==="
    log_warn "Mevcut chain durdurulacak (veri silinmeyecek)"

    for ip in "${ALL_IPS[@]}"; do
        log_info "Durduruluyor: $ip"
        ssh_cmd "$ip" '
            # Validator servislerini durdur
            systemctl list-units --type=service --state=running | grep -i pezkuwi | awk "{print \$1}" | xargs -r systemctl stop 2>/dev/null || true
            # Collator servislerini durdur
            systemctl list-units --type=service --state=running | grep -i "asset-hub\|people-chain" | awk "{print \$1}" | xargs -r systemctl stop 2>/dev/null || true
            # Doğrula
            sleep 2
            remaining=$(systemctl list-units --type=service --state=running | grep -ci "pezkuwi\|asset-hub\|people-chain" || true)
            echo "Kalan çalışan servis: $remaining"
        ' 2>/dev/null || log_warn "  $ip: bağlantı sorunu (atlanıyor)"
    done

    log_info "Tüm node'lar durduruldu"
}

# ===== FAZ: ARCHIVE =====
do_archive() {
    log_step "=== ESKİ CHAIN DİZİNLERİNİ ARCHIVE ET ==="

    for ip in "${ALL_IPS[@]}"; do
        log_info "Archive: $ip"
        ssh_cmd "$ip" "
            CHAIN_DIR='$REMOTE_DATA_DIR/$OLD_CHAIN_ID'
            ARCHIVE_DIR='$REMOTE_DATA_DIR/${ARCHIVE_SUFFIX}'
            if [ -d \"\$CHAIN_DIR\" ]; then
                mv \"\$CHAIN_DIR\" \"\$ARCHIVE_DIR\"
                echo '  Moved: $OLD_CHAIN_ID -> ${ARCHIVE_SUFFIX}'
            else
                echo '  Chain dizini bulunamadı (ilk kurulum olabilir)'
            fi

            # Teyrchain dizinlerini de archive et
            for tc_dir in asset-hub-pezkuwichain people-pezkuwichain; do
                TC_PATH='$REMOTE_DATA_DIR/'\$tc_dir
                if [ -d \"\$TC_PATH\" ]; then
                    mv \"\$TC_PATH\" \"\${TC_PATH}_${ARCHIVE_SUFFIX}\"
                    echo \"  Moved teyrchain: \$tc_dir\"
                fi
            done
        " 2>/dev/null || log_warn "  $ip: bağlantı sorunu"
    done

    log_info "Eski chain verileri archive edildi"
}

# ===== FAZ: DEPLOY =====
do_deploy() {
    log_step "=== BINARY + CHAIN SPEC DAĞIT ==="

    # Tüm VPS'lere relay binary + relay chain spec
    for ip in "${ALL_IPS[@]}"; do
        log_info "Deploy: $ip"

        # Binary
        scp_file "$PEZKUWI_BIN" "$ip" "$REMOTE_BIN_DIR/pezkuwi"
        ssh_cmd "$ip" "chmod +x $REMOTE_BIN_DIR/pezkuwi"

        # Relay chain spec
        ssh_cmd "$ip" "mkdir -p $REMOTE_SPEC_DIR"
        scp_file "$CHAINSPEC_DIR/relay-raw.json" "$ip" "$REMOTE_SPEC_DIR/relay-raw.json"

        echo "  Binary + relay spec OK"
    done

    # Collator VPS'lere teyrchain binary + chain spec'leri
    for ip in "${ALL_COLLATOR_IPS[@]}"; do
        log_info "Deploy teyrchain: $ip"

        scp_file "$TEYRCHAIN_BIN" "$ip" "$REMOTE_BIN_DIR/pezkuwi-teyrchain"
        ssh_cmd "$ip" "chmod +x $REMOTE_BIN_DIR/pezkuwi-teyrchain"

        scp_file "$CHAINSPEC_DIR/asset-hub-raw.json" "$ip" "$REMOTE_SPEC_DIR/asset-hub-raw.json"
        scp_file "$CHAINSPEC_DIR/people-raw.json" "$ip" "$REMOTE_SPEC_DIR/people-raw.json"

        echo "  Teyrchain binary + specs OK"
    done

    log_info "Deploy tamamlandı"
}

# ===== FAZ: INJECT =====
do_inject() {
    log_step "=== SESSION KEY'LERİ INJECT ET ==="

    # Python ile wallet JSON'dan key injection komutları oluştur ve çalıştır
    python3 << 'PYEOF'
import json
import subprocess
import sys
import time

WALLETS_FILE = "/home/mamostehp/res/MAINNET_WALLETS_20260128_235407.json"

# VPS -> Validator mapping (validator numarası -> VPS IP)
# Her VPS'te validator'lar farklı RPC portlarında çalışır
# İlk validator 9944, ikincisi 9945, üçüncüsü 9946, dördüncüsü 9947
VPS_VALIDATOR_MAP = {
    "62.146.235.186": [5, 6, 7, 21],
    "217.77.6.126": [1, 2, 3, 4],
    "217.77.15.51": [14],
    "161.97.183.44": [13],
    "161.97.185.100": [12],
    "109.123.229.159": [16],
    "161.97.116.241": [17],
    "46.250.241.121": [18],
    "164.68.121.181": [19],
    "158.220.93.23": [20],
    "207.180.194.103": [11],
    "167.86.70.241": [10],
    "167.86.108.190": [9],
    "207.180.233.147": [8],
    "178.18.252.120": [15],
}

# Collator mapping: (IP, port) -> collator name
COLLATOR_MAP = {
    ("217.77.6.126", 40944): "Asset_Hub_Collator_Azad",
    ("217.77.6.126", 41944): "People_Chain_Collator_Erin",
    ("173.249.57.228", 40944): "Asset_Hub_Collator_Beritan",
    ("173.249.57.228", 41944): "People_Chain_Collator_Firaz",
}

# Key type mapping (substrate key_type_id)
KEY_TYPES = {
    "babe": "babe",
    "grandpa": "gran",
    "para_validator": "para",
    "para_assignment": "asgn",
    "authority_discovery": "audi",
    "beefy": "beef",
}

with open(WALLETS_FILE) as f:
    data = json.load(f)

wallets = {w["name"]: w for w in data["wallets"]}

def inject_key(ip, port, key_type_id, seed, public_key):
    """Inject a single key via RPC"""
    payload = json.dumps({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "author_insertKey",
        "params": [key_type_id, seed, public_key]
    })
    cmd = [
        "ssh", "-o", "ConnectTimeout=10", "-o", "StrictHostKeyChecking=no",
        f"root@{ip}",
        f"curl -sH 'Content-Type: application/json' -d '{payload}' http://localhost:{port}"
    ]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
        if '"result"' in result.stdout:
            return True
        else:
            print(f"    WARN: {result.stdout.strip()}", file=sys.stderr)
            return False
    except Exception as e:
        print(f"    ERROR: {e}", file=sys.stderr)
        return False

def inject_validator_keys(ip, port, validator_num):
    """Inject all 6 session keys for a validator"""
    session_name = f"Validator_{validator_num:02d}_Session"
    session = wallets.get(session_name)
    if not session:
        print(f"  ERROR: {session_name} bulunamadı!")
        return False

    seed = session["master_seed"]
    keys = session["keys"]
    success = True

    for key_name, key_type_id in KEY_TYPES.items():
        public_key = keys[key_name]["public_key"]
        ok = inject_key(ip, port, key_type_id, seed, public_key)
        status = "OK" if ok else "FAIL"
        print(f"    {key_type_id}: {status}")
        if not ok:
            success = False

    return success

def inject_collator_key(ip, port, collator_name):
    """Inject aura key for a collator"""
    collator = wallets.get(collator_name)
    if not collator:
        print(f"  ERROR: {collator_name} bulunamadı!")
        return False

    seed = collator["seed_phrase"]
    public_key = collator["public_key"]
    ok = inject_key(ip, port, "aura", seed, public_key)
    status = "OK" if ok else "FAIL"
    print(f"    aura: {status}")
    return ok

# === Validator key injection ===
print("=== VALIDATOR SESSION KEY INJECTION ===")
total_ok = 0
total_fail = 0

for ip, validators in sorted(VPS_VALIDATOR_MAP.items()):
    for idx, val_num in enumerate(validators):
        port = 9944 + idx  # Her validator farklı RPC portu
        print(f"\n  Validator {val_num:02d} @ {ip}:{port}")
        if inject_validator_keys(ip, port, val_num):
            total_ok += 1
        else:
            total_fail += 1

print(f"\nValidator sonuç: {total_ok} OK, {total_fail} FAIL")

# === Collator key injection ===
print("\n=== COLLATOR KEY INJECTION ===")
for (ip, port), collator_name in sorted(COLLATOR_MAP.items()):
    print(f"\n  {collator_name} @ {ip}:{port}")
    inject_collator_key(ip, port, collator_name)

print("\nKey injection tamamlandı!")
PYEOF

    log_info "Session key injection tamamlandı"
}

# ===== FAZ: START =====
do_start() {
    log_step "=== NODE'LARI SIRALI BAŞLAT ==="

    # 1. Bootnode (VPS3 - Validator 1)
    log_info "1/4: Bootnode başlatılıyor (VPS3 - $BOOTNODE_IP)"
    ssh_cmd "$BOOTNODE_IP" '
        systemctl start pezkuwi-validator-1.service
        sleep 5
        systemctl start pezkuwi-validator-2.service
        systemctl start pezkuwi-validator-3.service
        systemctl start pezkuwi-validator-4.service
    ' 2>/dev/null
    log_info "  VPS3 validator'ları başlatıldı, 30 saniye bekleniyor..."
    sleep 30

    # 2. VPS2 validator'ları
    log_info "2/4: VPS2 validator'ları başlatılıyor (62.146.235.186)"
    ssh_cmd "62.146.235.186" '
        systemctl start pezkuwi-validator-5.service
        systemctl start pezkuwi-validator-6.service
        systemctl start pezkuwi-validator-7.service
        systemctl start pezkuwi-validator-21.service
    ' 2>/dev/null
    log_info "  VPS2 başlatıldı, 15 saniye bekleniyor..."
    sleep 15

    # 3. Kalan validator VPS'leri
    log_info "3/4: Kalan validator'lar başlatılıyor..."
    for ip in "${ALL_VALIDATOR_IPS[@]}"; do
        # VPS3 ve VPS2 zaten başlatıldı
        if [ "$ip" = "$BOOTNODE_IP" ] || [ "$ip" = "62.146.235.186" ]; then
            continue
        fi
        ssh_cmd "$ip" '
            for svc in $(systemctl list-unit-files | grep pezkuwi-validator | awk "{print \$1}"); do
                systemctl start "$svc" 2>/dev/null || true
            done
        ' 2>/dev/null &
    done
    wait
    log_info "  Tüm validator'lar başlatıldı"

    # 4. Relay chain'in finalize etmesini bekle
    log_info "4/4: Relay chain finalizasyon bekleniyor (60 saniye)..."
    sleep 60

    # Sağlık kontrolü
    local health
    health=$(ssh_cmd "$BOOTNODE_IP" "curl -sH 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"system_health\",\"params\":[]}' http://localhost:9944" 2>/dev/null)
    echo "  Relay health: $health"

    log_info "Relay chain başlatıldı!"
    log_warn "Collator'ları başlatmadan önce relay chain'in finalize ettiğini doğrulayın."
    echo ""
    echo "Collator'ları başlatmak için:"
    echo "  ssh root@$BOOTNODE_IP 'systemctl start asset-hub-azad.service && systemctl start people-chain-erin.service'"
    echo "  ssh root@173.249.57.228 'systemctl start asset-hub-beritan.service && systemctl start people-chain-firaz.service'"
}

# ===== FAZ: VERIFY =====
do_verify() {
    log_step "=== SAĞLIK KONTROLÜ ==="

    echo ""
    log_info "Relay Chain:"
    ssh_cmd "$BOOTNODE_IP" "
        echo '  Health:'
        curl -sH 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"system_health\",\"params\":[]}' http://localhost:9944
        echo ''
        echo '  Version:'
        curl -sH 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"state_getRuntimeVersion\",\"params\":[]}' http://localhost:9944 | python3 -c 'import json,sys; d=json.load(sys.stdin); print(\"  specVersion:\", d[\"result\"][\"specVersion\"])'
        echo '  Header:'
        curl -sH 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"chain_getHeader\",\"params\":[]}' http://localhost:9944 | python3 -c 'import json,sys; d=json.load(sys.stdin); print(\"  Block #\", int(d[\"result\"][\"number\"], 16))'
    " 2>/dev/null

    echo ""
    log_info "Asset Hub:"
    ssh_cmd "$BOOTNODE_IP" "
        curl -sH 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"system_health\",\"params\":[]}' http://localhost:40944
        echo ''
        curl -sH 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"chain_getHeader\",\"params\":[]}' http://localhost:40944 | python3 -c 'import json,sys; d=json.load(sys.stdin); print(\"  Block #\", int(d[\"result\"][\"number\"], 16))' 2>/dev/null || echo '  (henüz blok yok)'
    " 2>/dev/null

    echo ""
    log_info "People Chain:"
    ssh_cmd "$BOOTNODE_IP" "
        curl -sH 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"system_health\",\"params\":[]}' http://localhost:41944
        echo ''
        curl -sH 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"chain_getHeader\",\"params\":[]}' http://localhost:41944 | python3 -c 'import json,sys; d=json.load(sys.stdin); print(\"  Block #\", int(d[\"result\"][\"number\"], 16))' 2>/dev/null || echo '  (henüz blok yok)'
    " 2>/dev/null

    echo ""
    log_info "Doğrulama tamamlandı"
}

# ===== MAIN =====
PHASE="${1:-help}"

case "$PHASE" in
    check)   do_check ;;
    stop)    do_stop ;;
    archive) do_archive ;;
    deploy)  do_deploy ;;
    inject)  do_inject ;;
    start)   do_start ;;
    verify)  do_verify ;;
    all)
        do_check
        echo ""
        read -p "Devam etmek istiyor musunuz? (y/N) " confirm
        [ "$confirm" = "y" ] || exit 0
        do_stop
        do_archive
        do_deploy
        echo ""
        log_warn "Node'ları başlatıp key inject etmeniz gerekiyor."
        log_warn "Önce: bash tools/deploy-mainnet.sh start"
        log_warn "Node'lar ayağa kalkınca: bash tools/deploy-mainnet.sh inject"
        log_warn "Sonra: bash tools/deploy-mainnet.sh verify"
        ;;
    *)
        echo "Kullanım: $0 {check|stop|archive|deploy|inject|start|verify|all}"
        echo ""
        echo "Fazlar:"
        echo "  check   - Ön kontroller"
        echo "  stop    - Tüm node'ları durdur"
        echo "  archive - Eski chain verilerini yedekle"
        echo "  deploy  - Binary + chain spec dağıt"
        echo "  inject  - Session key inject (node'lar çalışırken)"
        echo "  start   - Node'ları sıralı başlat"
        echo "  verify  - Sağlık kontrolü"
        echo "  all     - check + stop + archive + deploy"
        ;;
esac
