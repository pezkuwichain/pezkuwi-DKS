# This is the build stage for Pezkuwi. Here we create the binary in a temporary image.
FROM docker.io/paritytech/ci-unified:bullseye-1.88.0-2025-06-27-v202507112050 as builder

WORKDIR /pezkuwi
COPY . /pezkuwi

RUN cargo build --locked --release

# This is the 2nd stage: a very small image where we copy the Pezkuwi binary."
FROM docker.io/paritytech/base-bin:latest

LABEL description="Multistage Docker image for Pezkuwi: a platform for web3" \
	io.parity.image.type="builder" \
	io.parity.image.authors="chevdor@gmail.com, devops-team@parity.io" \
	io.parity.image.vendor="Parity Technologies" \
	io.parity.image.description="Pezkuwi: a platform for web3" \
	io.parity.image.source="https://github.com/pezkuwichain/pezkuwi-sdk/blob/${VCS_REF}/docker/dockerfiles/pezkuwi/pezkuwi_builder.Dockerfile" \
	io.parity.image.documentation="https://github.com/pezkuwichain/pezkuwi-sdk/"

COPY --from=builder /pezkuwi/target/release/pezkuwi /usr/local/bin

USER root
RUN useradd -m -u 1001 -U -s /bin/sh -d /pezkuwi pezkuwi && \
	mkdir -p /data /pezkuwi/.local/share && \
	chown -R pezkuwi:pezkuwi /data && \
	ln -s /data /pezkuwi/.local/share/pezkuwi && \
# unclutter and minimize the attack surface
	rm -rf /usr/bin /usr/sbin && \
# check if executable works in this container
	/usr/local/bin/pezkuwi --version

USER pezkuwi

EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/pezkuwi"]
