FROM docker.io/paritytech/ci-unified:bullseye-1.88.0-2025-06-27-v202511141243 AS builder

WORKDIR /pezkuwi
COPY . /pezkuwi

# Install build dependencies for wasm-opt-sys (binutils provides 'ar')
RUN apt-get update && apt-get install -y --no-install-recommends \
    binutils \
    build-essential \
    cmake \
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
