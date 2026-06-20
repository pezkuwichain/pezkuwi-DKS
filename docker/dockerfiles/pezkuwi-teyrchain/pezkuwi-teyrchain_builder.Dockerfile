# This file is sourced from https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/docker/dockerfiles/pezkuwi-teyrchain/pezkuwi-teyrchain_builder.Dockerfile
# This is the build stage for pezkuwi-teyrchain. Here we create the binary in a temporary image.
FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /pezcumulus
COPY . /pezcumulus

RUN cargo build --release --locked -p pezkuwi-teyrchain

# This is the 2nd stage: a very small image where we copy the Pezkuwi binary."
FROM docker.io/library/ubuntu:20.04

LABEL io.parity.image.type="builder" \
    io.parity.image.authors="devops-team@parity.io" \
    io.parity.image.vendor="Parity Technologies" \
    io.parity.image.description="Multistage Docker image for pezkuwi-teyrchain" \
    io.parity.image.source="https://github.com/pezkuwichain/pezkuwi-sdk/blob/${VCS_REF}/docker/dockerfiles/pezkuwi-teyrchain/pezkuwi-teyrchain_builder.Dockerfile" \
    io.parity.image.documentation="https://github.com/pezkuwichain/pezkuwi-sdk/tree/master/pezcumulus"

COPY --from=builder /pezcumulus/target/release/pezkuwi-teyrchain /usr/local/bin

RUN useradd -m -u 1000 -U -s /bin/sh -d /pezcumulus pezkuwi-teyrchain && \
    mkdir -p /data /pezcumulus/.local/share && \
    chown -R pezkuwi-teyrchain:pezkuwi-teyrchain /data && \
    ln -s /data /pezcumulus/.local/share/pezkuwi-teyrchain && \
# unclutter and minimize the attack surface
    rm -rf /usr/bin /usr/sbin && \
# check if executable works in this container
    /usr/local/bin/pezkuwi-teyrchain --version

USER pezkuwi-teyrchain

EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/pezkuwi-teyrchain"]
