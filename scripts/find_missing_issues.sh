#!/bin/bash
cd /home/mamostehp/pezkuwi-sdk

# Koddaki tüm upstream issue numaraları
CODE_ISSUES=$(grep -rho "github.com/pezkuwichain/pezkuwi-sdk/issues/[0-9]*" --include="*.rs" 2>/dev/null | \
  grep -v target | grep -v vendor | \
  sed 's|github.com/pezkuwichain/pezkuwi-sdk/issues/||' | \
  sort -n | uniq)

# Zaten tracking issue olanlar (mapping dosyasından)
TRACKED=""
if [ -f ".claude/issue_mapping.txt" ]; then
  TRACKED=$(grep -v "^#" .claude/issue_mapping.txt | awk '{print $1}' | tr '\n' ' ')
fi

echo "=== DURUM RAPORU ==="
echo ""
echo "Koddaki upstream referansları:"
echo "$CODE_ISSUES" | wc -w
echo ""
echo "Zaten tracking issue olanlar:"
echo "$TRACKED"
echo ""
echo "=== OLUŞTURULMASI GEREKEN ISSUE'LAR ==="

MISSING=""
for i in $CODE_ISSUES; do
  found=0
  for t in $TRACKED; do
    if [ "$i" = "$t" ]; then
      found=1
      break
    fi
  done
  if [ $found -eq 0 ]; then
    MISSING="$MISSING $i"
  fi
done

echo "$MISSING" | tr ' ' '\n' | grep -v '^$' | sort -n
echo ""
echo "Toplam eksik: $(echo $MISSING | wc -w)"
