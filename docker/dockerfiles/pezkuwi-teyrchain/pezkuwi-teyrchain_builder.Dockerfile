# This file is sourced from https://github.com/pezkuwichain/pezkuwi-DKS/blob/master/docker/dockerfiles/pezkuwi-teyrchain/pezkuwi-teyrchain_builder.Dockerfile
# This is the build stage for pezkuwi-teyrchain. Here we create the binary in a temporary image.
FROM docker.io/paritytech/ci-linux:production@sha256:dc9abf9e877c5bad94828245406dac8a186530e1ad6a1b5f2072e5e19e1f64b4 as builder

WORKDIR /pezcumulus
COPY . /pezcumulus

RUN cargo build --release --locked -p pezkuwi-teyrchain

# This is the 2nd stage: a very small image where we copy the Pezkuwi binary."
FROM docker.io/library/ubuntu:20.04@sha256:8feb4d8ca5354def3d8fce243717141ce31e2c428701f6682bd2fafe15388214

LABEL io.parity.image.type="builder" \
    io.parity.image.authors="devops-team@parity.io" \
    io.parity.image.vendor="Parity Technologies" \
    io.parity.image.description="Multistage Docker image for pezkuwi-teyrchain" \
    io.parity.image.source="https://github.com/pezkuwichain/pezkuwi-DKS/blob/${VCS_REF}/docker/dockerfiles/pezkuwi-teyrchain/pezkuwi-teyrchain_builder.Dockerfile" \
    io.parity.image.documentation="https://github.com/pezkuwichain/pezkuwi-DKS/tree/master/pezcumulus"

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
