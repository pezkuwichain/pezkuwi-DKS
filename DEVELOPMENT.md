# Development Guide

Quick reference for building and running Pezkuwi SDK locally.
For full documentation see the [Development Guide wiki](https://docs.pezkuwichain.io/).

## System Requirements

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 4 cores | 8+ cores |
| RAM | 8 GB | 16+ GB |
| Disk | 50 GB | 100+ GB SSD |
| OS | Ubuntu 22.04 / macOS 13+ | Ubuntu 24.04 |

### Dependencies (Ubuntu)

```bash
sudo apt-get install -y \
  build-essential git clang cmake pkg-config \
  libssl-dev protobuf-compiler
```

### Dependencies (macOS)

```bash
brew install cmake protobuf openssl
```

## Rust Setup

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
```

> Toolchain version is pinned in `rust-toolchain.toml` (currently 1.88.0).
> `rustup show` will install it automatically.

## Build

```bash
# Type-check entire workspace (fast)
cargo check --workspace --locked

# Build node binary
cargo build --release -p pezkuwi --locked

# Binaries produced:
#   target/release/pezkuwi
#   target/release/pezkuwi-execute-worker
#   target/release/pezkuwi-prepare-worker
```

## Run a Local Dev Node

```bash
./target/release/pezkuwi --dev
```

## Tests

```bash
# Unit tests
cargo test --workspace --lib --locked

# With nextest (parallel, faster)
cargo nextest run --workspace --locked

# Doc tests
cargo test --doc --workspace --locked
```

## Linting

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
taplo format --config .config/taplo.toml
```

## Docs

```bash
cargo doc --workspace --no-deps --open
```

## More

- [Architecture](https://docs.pezkuwichain.io/)
- [Custom Pallets](https://docs.pezkuwichain.io/)
- [Contributing](./CONTRIBUTING.md)

