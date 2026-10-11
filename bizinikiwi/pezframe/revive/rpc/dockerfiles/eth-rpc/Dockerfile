# Debian bullseye's repositories moved to archive.debian.org, so `apt-get update` on the old
# bullseye CI image fails with 404. The official Rust image on trixie (Debian stable) carries the
# toolchain rust-toolchain.toml pins; the build needs clang (bindgen) and protoc on top of it.
FROM docker.io/library/rust:1.96.0-trixie AS builder

WORKDIR /pezkuwi
COPY . /pezkuwi

# binutils provides 'ar' for wasm-opt-sys; clang/libclang for bindgen; protoc for prost builds.
RUN apt-get update && apt-get install -y --no-install-recommends \
    binutils \
    build-essential \
    clang \
    cmake \
    libclang-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

RUN cargo fetch
# SKIP_WASM_BUILD=1 avoids serde_core + build-std conflict (upstream issue: serde-rs/serde#3021)
RUN SKIP_WASM_BUILD=1 cargo build --locked --profile production -p pezpallet-revive-eth-rpc --bin eth-rpc

FROM docker.io/parity/base-bin:latest
COPY --from=builder /pezkuwi/target/production/eth-rpc /usr/local/bin

USER root
RUN useradd -m -u 1001 -U -s /bin/sh -d /pezkuwi pezkuwi && \
	rm -rf /usr/bin /usr/sbin && \
	/usr/local/bin/eth-rpc --help

USER pezkuwi

# 8545 is the default port for the RPC server
# 9616 is the default port for the prometheus metrics
EXPOSE 8545 9616
ENTRYPOINT ["/usr/local/bin/eth-rpc"]

# We call the help by default
CMD ["--help"]
