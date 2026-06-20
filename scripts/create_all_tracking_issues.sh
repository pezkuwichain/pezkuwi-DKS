#!/bin/bash
# Tüm eksik upstream tracking issue'larını oluşturur

set -e
cd /home/mamostehp/pezkuwi-sdk

REPO="pezkuwichain/pezkuwi-sdk"
LABEL="upstream-tracking"
TODAY=$(date +%Y-%m-%d)
NEXT_CHECK=$(date -d '+1 month' +%Y-%m-%d 2>/dev/null || date -v+1m +%Y-%m-%d 2>/dev/null || echo "2026-01-23")
MAPPING_FILE=".claude/issue_mapping.txt"
LOG_FILE=".claude/issue_creation.log"

# Eksik issue numaraları
MISSING_ISSUES=(8 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 31 32 33 34 35 36 37 38 39 40 41 43 44 45 47 48 49 50 51 53 55 57 60 74 76 77 78 79 80 81 82 83 84 86 87 88 89 90 91 92 93 94 96 97 98 99 100 101 102 103 104 105 106 107 108 109 110 111 112 113 115 116 117 118 119 120 121 122 123 124 125 126 127 128 129 130 131 132 133 134 135 136 139 140 141 142 143 144 145 146 147 148 149 150 151 152 153 154 155 156 157 158 159 160 161 162 163 164 165 166 167 168 169 170 171 172 173 174 175 176 177 178 179 180 181 182 183 184 185 186 187 188 189 190)

echo "========================================" | tee -a "$LOG_FILE"
echo "Upstream Tracking Issues Creator" | tee -a "$LOG_FILE"
echo "Started: $(date)" | tee -a "$LOG_FILE"
echo "Total to create: ${#MISSING_ISSUES[@]}" | tee -a "$LOG_FILE"
echo "========================================" | tee -a "$LOG_FILE"

create_issue() {
    local upstream_num=$1

    # Zaten oluşturulmuş mu kontrol et
    if grep -q "^$upstream_num " "$MAPPING_FILE" 2>/dev/null; then
        echo "  SKIP: #$upstream_num already exists" | tee -a "$LOG_FILE"
        return 0
    fi

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

    result=$(gh issue create \
        --repo "$REPO" \
        --title "[Upstream] Track Polkadot SDK #$upstream_num" \
        --body "$body" \
        --label "$LABEL" 2>&1)

    new_num=$(echo "$result" | grep -oE '/issues/[0-9]+' | grep -oE '[0-9]+')

    if [ -n "$new_num" ]; then
        echo "$upstream_num $new_num" >> "$MAPPING_FILE"
        echo "  OK: Created #$new_num for upstream #$upstream_num" | tee -a "$LOG_FILE"
        return 0
    else
        echo "  FAIL: upstream #$upstream_num - $result" | tee -a "$LOG_FILE"
        return 1
    fi
}

success=0
failed=0
skipped=0

for upstream_num in "${MISSING_ISSUES[@]}"; do
    echo "Processing upstream #$upstream_num..."

    if grep -q "^$upstream_num " "$MAPPING_FILE" 2>/dev/null; then
        ((skipped++))
        echo "  SKIP: Already tracked"
        continue
    fi

    if create_issue "$upstream_num"; then
        ((success++))
    else
        ((failed++))
    fi

    # Rate limit için bekleme
    sleep 0.5
done

echo "" | tee -a "$LOG_FILE"
echo "========================================" | tee -a "$LOG_FILE"
echo "COMPLETED" | tee -a "$LOG_FILE"
echo "  Success: $success" | tee -a "$LOG_FILE"
echo "  Failed: $failed" | tee -a "$LOG_FILE"
echo "  Skipped: $skipped" | tee -a "$LOG_FILE"
echo "  Mapping file: $MAPPING_FILE" | tee -a "$LOG_FILE"
echo "Finished: $(date)" | tee -a "$LOG_FILE"
echo "========================================" | tee -a "$LOG_FILE"
