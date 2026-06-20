#!/usr/bin/env bash
set -e

cargo build --release -p pezcumulus-test-service --bin test-teyrchain -p pezkuwi --bin pezkuwi-prepare-worker --bin pezkuwi-execute-worker --bin pezkuwi -p pezkuwi-teyrchain-bin --bin pezkuwi-teyrchain

RELEASE_DIR=$(dirname "$(cargo locate-project --workspace --message-format plain)")/target/release

export PATH=$RELEASE_DIR:$PATH
ZOMBIE_PROVIDER=native cargo test --release -p pezcumulus-zombienet-sdk-tests --features zombie-ci "$@"
