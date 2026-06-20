#!/bin/bash
# Publish crates with rate limit handling

CRATES=(
  # Level 0 remaining
  "pezsp-core-hashing-proc-macro"
  "pezsp-wasm-interface"
  # Level 1
  "pezsp-arithmetic"
  "pezsp-io"
  "pezsp-runtime-interface-proc-macro"
  "pezsp-runtime-interface"
  "pezsp-core"
  "pezsp-keyring"
  "pezsp-weights"
  "pezsp-version-proc-macro"
  "pezsp-version"
  "pezsp-application-crypto"
  "pezsp-runtime"
  "pezsp-staking"
  "pezsp-state-machine"
  "pezsp-trie"
  "pezsp-database"
  "pezsp-maybe-compressed-blob"
)

PUBLISHED=0
FAILED=0

for crate in "${CRATES[@]}"; do
  echo "========================================"
  echo "Publishing: $crate"
  echo "Time: $(date -u)"
  echo "========================================"
  
  OUTPUT=$(cargo publish -p "$crate" 2>&1)
  EXIT_CODE=$?
  
  if [ $EXIT_CODE -eq 0 ]; then
    echo "SUCCESS: $crate"
    ((PUBLISHED++))
  elif echo "$OUTPUT" | grep -q "429 Too Many Requests"; then
    # Extract wait time
    WAIT_UNTIL=$(echo "$OUTPUT" | grep -oP 'after \K[^o]+')
    echo "RATE LIMITED - waiting until $WAIT_UNTIL"
    echo "Sleeping 120 seconds..."
    sleep 120
    # Retry
    echo "Retrying $crate..."
    cargo publish -p "$crate" 2>&1
    if [ $? -eq 0 ]; then
      echo "SUCCESS on retry: $crate"
      ((PUBLISHED++))
    else
      echo "FAILED on retry: $crate"
      ((FAILED++))
    fi
  elif echo "$OUTPUT" | grep -q "already uploaded"; then
    echo "SKIPPED (already published): $crate"
  else
    echo "FAILED: $crate"
    echo "$OUTPUT"
    ((FAILED++))
  fi
  
  echo "Waiting 65 seconds..."
  sleep 65
done

echo "========================================"
echo "SUMMARY"
echo "Published: $PUBLISHED"
echo "Failed: $FAILED"
echo "========================================"
