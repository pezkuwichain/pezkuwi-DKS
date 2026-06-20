#!/bin/bash
cd /home/mamostehp/pezkuwi-sdk

MAPPING_FILE=".claude/issue_mapping.txt"

create_issue() {
    local i=$1
    echo "Creating tracking issue for upstream #$i..."

    result=$(gh issue create \
        --repo pezkuwichain/pezkuwi-sdk \
        --title "[Upstream] Track Polkadot SDK #$i" \
        --body "## Upstream Reference Tracking

**Upstream URL:** https://github.com/paritytech/polkadot-sdk/issues/$i
**Type:** Issue

---

### Status Tracking:

- [x] Pending - Upstream not yet resolved
- [ ] Resolved - Fix merged upstream
- [ ] Evaluated - Assessed if needed for PezkuwiChain
- [ ] Applied - Fix applied to our chain
- [ ] Closed - Upstream issue closed
- [ ] Skipped - Not relevant for us

**Last Check:** 2025-12-23
**Next Check:** 2026-01-23

---

### Notes:
Automatically created tracking issue for upstream Polkadot SDK reference." \
        --label "upstream-tracking" 2>&1)

    new_num=$(echo "$result" | grep -oE '/issues/[0-9]+' | grep -oE '[0-9]+')
    if [ -n "$new_num" ]; then
        echo "$i $new_num" >> "$MAPPING_FILE"
        echo "  Created #$new_num for upstream #$i"
    else
        echo "  Failed: $result"
    fi
    sleep 1
}

# Create remaining test issues
for i in 3 4 5 7; do
    create_issue $i
done

echo ""
echo "Mapping file contents:"
cat "$MAPPING_FILE"
