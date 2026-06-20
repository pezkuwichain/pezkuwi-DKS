#!/usr/bin/env -S bash -eux

export RUSTFLAGS="-Cdebug-assertions=y -Dwarnings"
cargo check --release
cargo check --release --features="bandersnatch-experimental"

export RUSTFLAGS="$RUSTFLAGS --cfg bizinikiwi_runtime"
T=wasm32v1-none
cargo check --release --target=$T --no-default-features
