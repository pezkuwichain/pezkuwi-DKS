#!/bin/bash
# Upstream Tracking Issues Creator
# Polkadot SDK issue'larını takip eden issue'lar oluşturur

set -e

REPO="pezkuwichain/pezkuwi-sdk"
LABEL="upstream-tracking"
TODAY=$(date +%Y-%m-%d)
NEXT_CHECK=$(date -d '+1 month' +%Y-%m-%d 2>/dev/null || date -v+1m +%Y-%m-%d)

# Mapping dosyası - oluşturulan issue'ları kaydeder
MAPPING_FILE="/home/mamostehp/pezkuwi-sdk/.claude/issue_mapping.txt"

# Tüm upstream issue numaraları
ALL_ISSUES=(2 3 4 5 7 8 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 31 32 33 34 35 36 37 38 39 40 41 43 44 45 47 48 49 50 51 53 55 57 60 74 76 77 78 79 80 81 82 83 84 86 87 88 89 90 91 92 93 94 96 97 98 99 100 101 102 103 104 105 106 107 108 109 110 111 112 113 115 116 117 118 119 120 121 122 123 124 125 126 127 128 129 130 131 132 133 134 135 136 139 140 141 142 143 144 145 146 147 148 149 150 151 152 153 154 155 156 157 158 159 160 161 162 163 164 165 166 167 168 169 170 171 172 173 174 175 176 177 178 179 180 181 182 183 184 185 186 187 188 189 190)

# Test modu - sadece ilk 5 issue
TEST_ISSUES=(2 3 4 5 7)

create_issue() {
    local upstream_num=$1

    echo "Creating tracking issue for Polkadot SDK #$upstream_num..."

    # Issue body
    local body="## Upstream Reference Tracking

**Upstream URL:** https://github.com/paritytech/polkadot-sdk/issues/$upstream_num
**Type:** Issue

---

### Status Tracking:

- [x] Pending - Upstream not yet resolved
- [ ] Resolved - Fix merged upstream
- [ ] Evaluated - Assessed if needed for PezkuwiChain
- [ ] Applied - Fix applied to our chain
- [ ] Closed - Upstream issue closed
- [ ] Skipped - Not relevant for us

**Last Check:** $TODAY
**Next Check:** $NEXT_CHECK

---

### Notes:
Automatically created tracking issue for upstream Polkadot SDK reference.

This issue is tracked by our weekly bot that monitors upstream changes."

    # Issue oluştur ve numarasını al
    local result=$(gh issue create \
        --repo "$REPO" \
        --title "[Upstream] Track Polkadot SDK #$upstream_num" \
        --body "$body" \
        --label "$LABEL" 2>&1)

    # Issue URL'den numarayı çıkar
    local new_issue_num=$(echo "$result" | grep -oE '/issues/[0-9]+' | grep -oE '[0-9]+')

    if [ -n "$new_issue_num" ]; then
        echo "$upstream_num $new_issue_num" >> "$MAPPING_FILE"
        echo "  ✓ Created issue #$new_issue_num for upstream #$upstream_num"
        return 0
    else
        echo "  ✗ Failed to create issue for upstream #$upstream_num"
        echo "  Error: $result"
        return 1
    fi
}

ensure_label_exists() {
    echo "Checking if label '$LABEL' exists..."
    if ! gh label list --repo "$REPO" | grep -q "$LABEL"; then
        echo "Creating label '$LABEL'..."
        gh label create "$LABEL" \
            --repo "$REPO" \
            --description "Issues tracking upstream Polkadot SDK changes" \
            --color "0366d6"
        echo "  ✓ Label created"
    else
        echo "  ✓ Label already exists"
    fi
}

test_mode() {
    echo "========================================"
    echo "TEST MODE - Creating 5 tracking issues"
    echo "========================================"
    echo ""

    ensure_label_exists

    # Mapping dosyasını başlat
    echo "# Upstream Issue -> Pezkuwi Issue Mapping" > "$MAPPING_FILE"
    echo "# Format: UPSTREAM_NUM PEZKUWI_NUM" >> "$MAPPING_FILE"
    echo "# Created: $TODAY" >> "$MAPPING_FILE"
    echo "" >> "$MAPPING_FILE"

    local success=0
    local failed=0

    for issue_num in "${TEST_ISSUES[@]}"; do
        if create_issue "$issue_num"; then
            ((success++))
        else
            ((failed++))
        fi
        # Rate limit için kısa bekleme
        sleep 1
    done

    echo ""
    echo "========================================"
    echo "TEST COMPLETE"
    echo "  Success: $success"
    echo "  Failed: $failed"
    echo "  Mapping file: $MAPPING_FILE"
    echo "========================================"
    echo ""
    echo "Mapping içeriği:"
    cat "$MAPPING_FILE"
}

full_mode() {
    echo "========================================"
    echo "FULL MODE - Creating ALL tracking issues"
    echo "Total: ${#ALL_ISSUES[@]} issues"
    echo "========================================"
    echo ""

    ensure_label_exists

    # Mevcut mapping'i kontrol et
    if [ -f "$MAPPING_FILE" ]; then
        echo "Existing mapping file found. Skipping already created issues."
    else
        echo "# Upstream Issue -> Pezkuwi Issue Mapping" > "$MAPPING_FILE"
        echo "# Format: UPSTREAM_NUM PEZKUWI_NUM" >> "$MAPPING_FILE"
        echo "# Created: $TODAY" >> "$MAPPING_FILE"
        echo "" >> "$MAPPING_FILE"
    fi

    local success=0
    local failed=0
    local skipped=0

    for issue_num in "${ALL_ISSUES[@]}"; do
        # Zaten oluşturulmuş mu kontrol et
        if grep -q "^$issue_num " "$MAPPING_FILE" 2>/dev/null; then
            echo "Skipping upstream #$issue_num (already created)"
            ((skipped++))
            continue
        fi

        if create_issue "$issue_num"; then
            ((success++))
        else
            ((failed++))
        fi
        # Rate limit için kısa bekleme
        sleep 1
    done

    echo ""
    echo "========================================"
    echo "FULL MODE COMPLETE"
    echo "  Success: $success"
    echo "  Failed: $failed"
    echo "  Skipped: $skipped"
    echo "  Total processed: $((success + failed + skipped))"
    echo "  Mapping file: $MAPPING_FILE"
    echo "========================================"
}

# Ana menu
case "${1:-}" in
    test)
        test_mode
        ;;
    full)
        full_mode
        ;;
    *)
        echo "Usage: $0 {test|full}"
        echo ""
        echo "  test  - Create 5 test issues to verify the script works"
        echo "  full  - Create all 143 tracking issues"
        echo ""
        echo "Prerequisites:"
        echo "  - GitHub CLI (gh) must be installed and authenticated"
        echo "  - Run 'gh auth status' to verify"
        ;;
esac
