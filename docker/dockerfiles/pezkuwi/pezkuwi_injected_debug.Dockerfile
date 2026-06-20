FROM docker.io/library/ubuntu:20.04

# metadata
ARG VCS_REF
ARG BUILD_DATE
ARG IMAGE_NAME

LABEL io.parity.image.authors="devops-team@parity.io" \
	io.parity.image.vendor="Parity Technologies" \
	io.parity.image.title="${IMAGE_NAME}" \
	io.parity.image.description="Pezkuwi: a platform for web3" \
	io.parity.image.source="https://github.com/pezkuwichain/pezkuwi-sdk/blob/${VCS_REF}/docker/dockerfiles/pezkuwi/pezkuwi_injected_debug.Dockerfile" \
	io.parity.image.revision="${VCS_REF}" \
	io.parity.image.created="${BUILD_DATE}" \
	io.parity.image.documentation="https://github.com/pezkuwichain/pezkuwi-sdk"

# show backtraces
ENV RUST_BACKTRACE 1

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
	libssl1.1 \
	ca-certificates && \
	# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
	# add user and link ~/.local/share/pezkuwi to /data
	useradd -m -u 1000 -U -s /bin/sh -d /pezkuwi pezkuwi && \
	mkdir -p /data /pezkuwi/.local/share /pezkuwi/runtimes && \
	chown -R pezkuwi:pezkuwi /data && \
	ln -s /data /pezkuwi/.local/share/pezkuwi

# add pezkuwi binaries to docker image
COPY ./artifacts/pezkuwi ./artifacts/pezkuwi-execute-worker ./artifacts/pezkuwi-prepare-worker /usr/local/bin/

# add runtime binaries to docker image
COPY ./artifacts/runtimes /pezkuwi/runtimes/

USER pezkuwi

# check if executable works in this container
RUN /usr/local/bin/pezkuwi --version
RUN /usr/local/bin/pezkuwi-execute-worker --version
RUN /usr/local/bin/pezkuwi-prepare-worker --version

EXPOSE 30333 9933 9944
VOLUME ["/pezkuwi"]

ENTRYPOINT ["/usr/local/bin/pezkuwi"]
