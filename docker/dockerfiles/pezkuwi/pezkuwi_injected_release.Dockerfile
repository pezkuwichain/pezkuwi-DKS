FROM docker.io/library/ubuntu:20.04

# metadata
ARG VCS_REF
ARG BUILD_DATE
ARG PEZKUWI_VERSION
ARG PEZKUWI_GPGKEY=9D4B2B6EB8F97156D19669A9FF0812D491B96798
ARG GPG_KEYSERVER="keyserver.ubuntu.com"

LABEL io.parity.image.authors="devops-team@parity.io" \
	io.parity.image.vendor="Parity Technologies" \
	io.parity.image.title="parity/pezkuwi" \
	io.parity.image.description="Pezkuwi: a platform for web3. This is the official Parity image with an injected binary." \
	io.parity.image.source="https://github.com/pezkuwichain/pezkuwi-sdk/blob/${VCS_REF}/docker/dockerfiles/pezkuwi/pezkuwi_injected_release.Dockerfile" \
	io.parity.image.revision="${VCS_REF}" \
	io.parity.image.created="${BUILD_DATE}" \
	io.parity.image.documentation="https://github.com/pezkuwichain/pezkuwi-sdk/"

# show backtraces
ENV RUST_BACKTRACE 1

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
		libssl1.1 \
		ca-certificates \
		gnupg && \
	useradd -m -u 1000 -U -s /bin/sh -d /pezkuwi pezkuwi && \
# add repo's gpg keys and install the published pezkuwi binary
	gpg --keyserver ${GPG_KEYSERVER} --recv-keys ${PEZKUWI_GPGKEY} && \
	gpg --export ${PEZKUWI_GPGKEY} > /usr/share/keyrings/parity.gpg && \
	echo 'deb [signed-by=/usr/share/keyrings/parity.gpg] https://releases.parity.io/deb release main' > /etc/apt/sources.list.d/parity.list && \
	apt-get update && \
	apt-get install -y --no-install-recommends pezkuwi=${PEZKUWI_VERSION#?} && \
# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	rm -rf /var/lib/apt/lists/* ; \
	mkdir -p /data /pezkuwi/.local/share && \
	chown -R pezkuwi:pezkuwi /data && \
	ln -s /data /pezkuwi/.local/share/pezkuwi

USER pezkuwi

# check if executable works in this container
RUN /usr/bin/pezkuwi --version
RUN /usr/local/bin/pezkuwi-execute-worker --version
RUN /usr/local/bin/pezkuwi-prepare-worker --version

EXPOSE 30333 9933 9944
VOLUME ["/pezkuwi"]

ENTRYPOINT ["/usr/bin/pezkuwi"]
