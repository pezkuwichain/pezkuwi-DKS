#!/bin/bash
# Tüm kalan upstream tracking issue'ları oluşturma scripti
# Issue #21'den başlayarak kalan tüm URL'leri işler

set -e

REPO="pezkuwichain/kurdistan-sdk"
CODEBASE="/home/mamostehp/kurdistan-sdk"
LABEL="upstream-tracking"

cd "$CODEBASE"

NEXT_ISSUE_NUM=21
TOTAL_PROCESSED=0
TOTAL_SKIPPED=0

# Satır 21'den başla, her URL'yi işle
tail -n +21 /tmp/upstream_urls.txt | while read -r URL; do
    # Boş satırları atla
    if [ -z "$URL" ]; then
        continue
    fi

    # URL'den bilgileri çıkar
    UPSTREAM_REPO=$(echo "$URL" | sed -E 's|.*paritytech/([^/]+)/.*|\1|')
    TYPE=$(echo "$URL" | sed -E 's|.*/([^/]+)/[0-9]+$|\1|')
    NUM=$(echo "$URL" | sed -E 's|.*/([0-9]+)$|\1|')

    # Orijinal upstream repo adını belirle (rebranding düzelt)
    ORIGINAL_REPO="$UPSTREAM_REPO"
    case "$UPSTREAM_REPO" in
        "bizinikiwi")
            ORIGINAL_REPO="substrate"
            ;;
        "pezcumulus")
            ORIGINAL_REPO="cumulus"
            ;;
        "pezkuwi")
            ORIGINAL_REPO="polkadot"
            ;;
    esac

    echo ""
    echo "=========================================="
    echo "Processing Issue #$NEXT_ISSUE_NUM"
    echo "URL: $URL"
    echo "Repo: paritytech/$ORIGINAL_REPO, Type: $TYPE, Num: $NUM"
    echo "=========================================="

    # Aranacak pattern'i oluştur
    SEARCH_PATTERN="paritytech/$UPSTREAM_REPO/$TYPE/$NUM"

    # Dosyaları bul
    FILES=$(grep -rl "$SEARCH_PATTERN" --include="*.rs" --include="*.md" --include="*.toml" . 2>/dev/null | grep -v "/target/" | grep -v "/.git/" || true)

    if [ -z "$FILES" ]; then
        echo "UYARI: $SEARCH_PATTERN için dosya bulunamadı, atlıyorum..."
        TOTAL_SKIPPED=$((TOTAL_SKIPPED + 1))
        continue
    fi

    echo "Bulunan dosyalar:"
    echo "$FILES"
    FILE_COUNT=$(echo "$FILES" | wc -l)
    echo "Toplam: $FILE_COUNT dosya"

    # GitHub issue oluştur
    ISSUE_BODY="**Upstream $TYPE:** https://github.com/paritytech/$ORIGINAL_REPO/$TYPE/$NUM

**Status Tracking:**
- [x] Pending - Upstream not yet resolved
- [ ] Resolved - Fix merged upstream
- [ ] Evaluated - Assessed if needed for Kurdistan SDK
- [ ] Applied - Fix applied to our chain
- [ ] Closed - Upstream $TYPE closed
- [ ] Skipped - Not relevant for us

**Last Check:** $(date +%Y-%m-%d)
**Next Check:** $(date -d '+1 month' +%Y-%m-%d 2>/dev/null || date -v+1m +%Y-%m-%d 2>/dev/null || echo 'TBD')

**Notes:**
Tracking upstream paritytech/$ORIGINAL_REPO#$NUM.
Periodically check upstream and update checkboxes above based on status changes."

    echo "Creating GitHub issue..."
    CREATED_ISSUE=$(gh issue create \
        --repo "$REPO" \
        --label "$LABEL" \
        --title "[Upstream Tracking] paritytech/$ORIGINAL_REPO#$NUM" \
        --body "$ISSUE_BODY")

    NEW_ISSUE_NUM=$(echo "$CREATED_ISSUE" | grep -oE '[0-9]+$')
    echo "Created: Issue #$NEW_ISSUE_NUM"

    # TÜM dosyalardaki URL'leri güncelle
    echo "Updating ALL files..."
    for FILE in $FILES; do
        echo "  Updating: $FILE"
        sed -i "s|$SEARCH_PATTERN|pezkuwichain/kurdistan-sdk/issues/$NEW_ISSUE_NUM|g" "$FILE"
    done

    # Doğrulama
    REMAINING=$(grep -rl "$SEARCH_PATTERN" --include="*.rs" --include="*.md" --include="*.toml" . 2>/dev/null | grep -v "/target/" | grep -v "/.git/" || true)

    if [ -n "$REMAINING" ]; then
        echo "HATA: Hala eski URL kalmış dosyalar var:"
        echo "$REMAINING"
        exit 1
    fi

    echo "Issue #$NEW_ISSUE_NUM tamamlandı ($FILE_COUNT dosya güncellendi)"

    NEXT_ISSUE_NUM=$((NEXT_ISSUE_NUM + 1))
    TOTAL_PROCESSED=$((TOTAL_PROCESSED + 1))
done

echo ""
echo "=========================================="
echo "TAMAMLANDI!"
echo "=========================================="
