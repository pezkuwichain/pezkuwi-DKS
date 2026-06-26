FROM docker.io/paritytech/ci-unified:latest as builder

WORKDIR /pezkuwi
COPY . /pezkuwi

RUN cargo fetch
RUN cargo build --workspace --locked --release

FROM docker.io/parity/base-bin:latest

COPY --from=builder /pezkuwi/target/release/minimal-template-node /usr/local/bin

USER root
RUN useradd -m -u 1001 -U -s /bin/sh -d /pezkuwi pezkuwi && \
	mkdir -p /data /pezkuwi/.local/share && \
	chown -R pezkuwi:pezkuwi /data && \
	ln -s /data /pezkuwi/.local/share/pezkuwi && \
# unclutter and minimize the attack surface
	rm -rf /usr/bin /usr/sbin && \
# check if executable works in this container
	/usr/local/bin/minimal-template-node --version

USER pezkuwi

EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/minimal-template-node"]
