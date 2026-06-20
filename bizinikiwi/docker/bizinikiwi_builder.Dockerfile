# This is the build stage for Bizinikiwi. Here we create the binary.
FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /bizinikiwi
COPY . /bizinikiwi
RUN cargo build --locked --release

# This is the 2nd stage: a very small image where we copy the Bizinikiwi binary."
FROM docker.io/library/ubuntu:20.04
LABEL description="Multistage Docker image for Bizinikiwi: a platform for web3" \
	io.parity.image.type="builder" \
	io.parity.image.authors="chevdor@gmail.com, devops-team@parity.io" \
	io.parity.image.vendor="Parity Technologies" \
	io.parity.image.description="Bizinikiwi is a next-generation framework for blockchain innovation" \
	io.parity.image.source="https://github.com/pezkuwichain/pezkuwi-sdk/blob/${VCS_REF}/bizinikiwi/docker/bizinikiwi_builder.Dockerfile" \
	io.parity.image.documentation="https://github.com/pezkuwichain/pezkuwi-sdk"

COPY --from=builder /bizinikiwi/target/release/bizinikiwi /usr/local/bin
COPY --from=builder /bizinikiwi/target/release/subkey /usr/local/bin
COPY --from=builder /bizinikiwi/target/release/node-template /usr/local/bin
COPY --from=builder /bizinikiwi/target/release/chain-spec-builder /usr/local/bin

RUN useradd -m -u 1000 -U -s /bin/sh -d /bizinikiwi bizinikiwi && \
	mkdir -p /data /bizinikiwi/.local/share/bizinikiwi && \
	chown -R bizinikiwi:bizinikiwi /data && \
	ln -s /data /bizinikiwi/.local/share/bizinikiwi && \
# Sanity checks
	ldd /usr/local/bin/bizinikiwi && \
# unclutter and minimize the attack surface
	rm -rf /usr/bin /usr/sbin && \
	/usr/local/bin/bizinikiwi --version

USER bizinikiwi
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]
