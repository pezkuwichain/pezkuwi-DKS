#!/bin/bash
# Koddaki issue linklerini yeni numaralara günceller

cd /home/mamostehp/pezkuwi-sdk

MAPPING_FILE=".claude/issue_mapping.txt"

echo "=== Updating issue links in code ==="

# Mapping dosyasını oku (header'ları atla)
while read -r upstream_num pezkuwi_num; do
  # Boş satır veya yorum satırını atla
  [[ -z "$upstream_num" || "$upstream_num" == \#* ]] && continue

  # Sadece sayıları işle
  [[ ! "$upstream_num" =~ ^[0-9]+$ ]] && continue

  # Değişiklik yapılacak dosyaları bul
  files=$(grep -rl "pezkuwi-sdk/issues/$upstream_num[^0-9]" --include="*.rs" . 2>/dev/null | grep -v target | grep -v vendor | head -20)

  if [ -n "$files" ]; then
    echo "Upstream #$upstream_num -> Pezkuwi #$pezkuwi_num"
    for f in $files; do
      # Tam URL'yi değiştir (sadece tam eşleşme)
      sed -i "s|pezkuwichain/pezkuwi-sdk/issues/$upstream_num\([^0-9]\)|pezkuwichain/pezkuwi-sdk/issues/$pezkuwi_num\1|g" "$f"
      sed -i "s|pezkuwichain/pezkuwi-sdk/issues/$upstream_num\$|pezkuwichain/pezkuwi-sdk/issues/$pezkuwi_num|g" "$f"
    done
  fi
done < "$MAPPING_FILE"

echo ""
echo "=== Done ==="
