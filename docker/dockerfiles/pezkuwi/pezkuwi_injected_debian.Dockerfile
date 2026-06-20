FROM docker.io/paritytech/base-bin

# metadata
ARG VCS_REF
ARG BUILD_DATE
ARG PEZKUWI_VERSION

LABEL io.parity.image.authors="devops-team@parity.io" \
	io.parity.image.vendor="Parity Technologies" \
	io.parity.image.title="parity/pezkuwi" \
	io.parity.image.description="Pezkuwi: a platform for web3. This is the official Parity image with an injected binary." \
	io.parity.image.source="https://github.com/pezkuwichain/pezkuwi-sdk/blob/${VCS_REF}/docker/dockerfiles/pezkuwi/pezkuwi_injected_debian.Dockerfile" \
	io.parity.image.revision="${VCS_REF}" \
	io.parity.image.created="${BUILD_DATE}" \
	io.parity.image.documentation="https://github.com/pezkuwichain/pezkuwi-sdk/"

USER root

# show backtraces
ENV RUST_BACKTRACE 1

RUN \
	apt-get update && \
	apt-get install -y --no-install-recommends pezkuwi=${PEZKUWI_VERSION#?} && \
	apt-get autoremove -y && \
	apt-get clean && \
	rm -rf /var/lib/apt/lists/* ; \
	mkdir -p /data /pezkuwi/.local/share && \
	chown -R parity:parity /data && \
	ln -s /data /pezkuwi/.local/share/pezkuwi

USER parity

# check if executable works in this container
RUN /usr/bin/pezkuwi --version
RUN /usr/lib/pezkuwi/pezkuwi-execute-worker --version
RUN /usr/lib/pezkuwi/pezkuwi-prepare-worker --version

EXPOSE 30333 9933 9944 9615
VOLUME ["/pezkuwi"]

ENTRYPOINT ["/usr/bin/pezkuwi"]
