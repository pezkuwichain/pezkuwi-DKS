#!/bin/bash
cd /home/mamostehp/pezkuwi-sdk
LOG="/home/mamostehp/pezkuwi-sdk/publish.log"

publish_crate() {
  local crate=$1
  echo "$(date -u) - Publishing $crate" | tee -a $LOG
  
  OUTPUT=$(cargo publish -p "$crate" 2>&1)
  EXIT_CODE=$?
  
  if [ $EXIT_CODE -eq 0 ]; then
    echo "$(date -u) - SUCCESS: $crate" | tee -a $LOG
    return 0
  elif echo "$OUTPUT" | grep -q "429 Too Many Requests"; then
    echo "$(date -u) - RATE LIMITED: $crate - waiting 120s" | tee -a $LOG
    sleep 120
    # Retry
    OUTPUT=$(cargo publish -p "$crate" 2>&1)
    if [ $? -eq 0 ]; then
      echo "$(date -u) - SUCCESS (retry): $crate" | tee -a $LOG
      return 0
    fi
  elif echo "$OUTPUT" | grep -q "already uploaded"; then
    echo "$(date -u) - SKIPPED: $crate (already published)" | tee -a $LOG
    return 0
  fi
  
  echo "$(date -u) - FAILED: $crate" | tee -a $LOG
  echo "$OUTPUT" >> $LOG
  return 1
}

# Level 1 crates
CRATES=(
  "pezsp-arithmetic"
  "pezsp-runtime-interface-proc-macro"
  "pezsp-runtime-interface"
  "pezsp-io"
  "pezsp-core"
  "pezsp-keyring"
  "pezsp-weights"
  "pezsp-version-proc-macro"
  "pezsp-version"
  "pezsp-application-crypto"
  "pezsp-metadata-ir"
)

for crate in "${CRATES[@]}"; do
  publish_crate "$crate"
  sleep 65
done

echo "Batch complete!" | tee -a $LOG
