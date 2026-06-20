#!/bin/bash
# Upstream tracking issue oluşturma scripti v2
# Issue #16-#20 için test
# FARK: Tüm dosyaları güncelliyor, sadece ilkini değil

set -e

REPO="pezkuwichain/kurdistan-sdk"
CODEBASE="/home/mamostehp/kurdistan-sdk"
LABEL="upstream-tracking"

# Issue #16-#20 için upstream URL'ler (satır 16-20)
ISSUES=(
    "14246:substrate:14246"
    "14425:substrate:14425"
    "1444:substrate:1444"
    "1458:substrate:1458"
    "14622:substrate:14622"
)

NEXT_ISSUE_NUM=16

cd "$CODEBASE"

for entry in "${ISSUES[@]}"; do
    IFS=':' read -r ISSUE_NUM UPSTREAM_REPO UPSTREAM_ISSUE <<< "$entry"

    echo ""
    echo "=========================================="
    echo "Processing Issue #$NEXT_ISSUE_NUM"
    echo "Upstream: paritytech/$UPSTREAM_REPO#$UPSTREAM_ISSUE"
    echo "=========================================="

    # 1. Tüm dosyaları bul (target ve .git hariç)
    FILES=$(grep -rl "paritytech/bizinikiwi/issues/$ISSUE_NUM" --include="*.rs" --include="*.md" --include="*.toml" . 2>/dev/null | grep -v "/target/" | grep -v "/.git/" || true)

    if [ -z "$FILES" ]; then
        echo "UYARI: $ISSUE_NUM için dosya bulunamadı, atlıyorum..."
        continue
    fi

    echo "Bulunan dosyalar:"
    echo "$FILES"
    FILE_COUNT=$(echo "$FILES" | wc -l)
    echo "Toplam: $FILE_COUNT dosya"

    # 2. GitHub issue oluştur
    ISSUE_BODY="**Upstream Issue:** https://github.com/paritytech/$UPSTREAM_REPO/issues/$UPSTREAM_ISSUE

**Status Tracking:**
- [x] Pending - Upstream not yet resolved
- [ ] Resolved - Fix merged upstream
- [ ] Evaluated - Assessed if needed for Kurdistan SDK
- [ ] Applied - Fix applied to our chain
- [ ] Closed - Upstream issue closed
- [ ] Skipped - Not relevant for us

**Last Check:** $(date +%Y-%m-%d)
**Next Check:** $(date -d '+1 month' +%Y-%m-%d 2>/dev/null || date -v+1m +%Y-%m-%d 2>/dev/null || echo 'TBD')

**Notes:**
Tracking upstream issue paritytech/$UPSTREAM_REPO#$UPSTREAM_ISSUE.
Periodically check upstream and update checkboxes above based on status changes."

    echo "Creating GitHub issue..."
    CREATED_ISSUE=$(gh issue create \
        --repo "$REPO" \
        --label "$LABEL" \
        --title "[Upstream Tracking] paritytech/$UPSTREAM_REPO#$UPSTREAM_ISSUE" \
        --body "$ISSUE_BODY")

    NEW_ISSUE_NUM=$(echo "$CREATED_ISSUE" | grep -oE '[0-9]+$')
    echo "Created: Issue #$NEW_ISSUE_NUM"

    # 3. TÜM dosyalardaki URL'leri güncelle (önemli fark!)
    echo "Updating ALL files..."
    for FILE in $FILES; do
        echo "  Updating: $FILE"
        sed -i "s|paritytech/bizinikiwi/issues/$ISSUE_NUM|pezkuwichain/kurdistan-sdk/issues/$NEW_ISSUE_NUM|g" "$FILE"
    done

    # 4. Doğrulama - eski URL kaldı mı?
    REMAINING=$(grep -rl "paritytech/bizinikiwi/issues/$ISSUE_NUM" --include="*.rs" --include="*.md" --include="*.toml" . 2>/dev/null | grep -v "/target/" | grep -v "/.git/" || true)

    if [ -n "$REMAINING" ]; then
        echo "HATA: Hala eski URL kalmış dosyalar var:"
        echo "$REMAINING"
        exit 1
    fi

    echo "Issue #$NEW_ISSUE_NUM tamamlandı ($FILE_COUNT dosya güncellendi)"

    NEXT_ISSUE_NUM=$((NEXT_ISSUE_NUM + 1))
done

echo ""
echo "=========================================="
echo "TAMAMLANDI!"
echo "Issue #16-#20 oluşturuldu ve dosyalar güncellendi"
echo "=========================================="
