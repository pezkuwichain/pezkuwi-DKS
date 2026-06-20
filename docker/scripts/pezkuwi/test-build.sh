#!/usr/bin/env bash

TMP=$(mktemp -d)
ENGINE=${ENGINE:-podman}

# You need to build an injected image first

# Fetch some binaries
$ENGINE run --user root --rm -i \
  -v "$TMP:/export" \
  --entrypoint /bin/bash \
  parity/pezkuwi -c \
  'cp "$(which pezkuwi)" /export'

echo "Checking binaries we got:"
tree $TMP

./build-injected.sh $TMP
