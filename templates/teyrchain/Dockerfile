FROM docker.io/paritytech/ci-unified:latest@sha256:e2b580514e943dd979e08bc69943f51515dab1eceef2c9ad2fddffe364aa1377 as builder

WORKDIR /pezkuwi
COPY . /pezkuwi

RUN cargo fetch
RUN cargo build --workspace --locked --profile production

FROM docker.io/parity/base-bin:latest@sha256:2391554e30053b07d0c22127df137b66e48282c201ec8e1e4634e3ffd29ec6fd

COPY --from=builder /pezkuwi/target/production/teyrchain-template-node /usr/local/bin

USER root
RUN useradd -m -u 1001 -U -s /bin/sh -d /pezkuwi pezkuwi && \
	mkdir -p /data /pezkuwi/.local/share && \
	chown -R pezkuwi:pezkuwi /data && \
	ln -s /data /pezkuwi/.local/share/pezkuwi && \
# unclutter and minimize the attack surface
	rm -rf /usr/bin /usr/sbin && \
# check if executable works in this container
	/usr/local/bin/teyrchain-template-node --version

USER pezkuwi

EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/teyrchain-template-node"]
