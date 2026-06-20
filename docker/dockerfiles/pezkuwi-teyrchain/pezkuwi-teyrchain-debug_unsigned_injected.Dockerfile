FROM docker.io/library/ubuntu:20.04

# metadata
ARG VCS_REF
ARG BUILD_DATE
ARG IMAGE_NAME

LABEL io.parity.image.authors="devops-team@parity.io" \
	io.parity.image.vendor="Parity Technologies" \
	io.parity.image.title="${IMAGE_NAME}" \
	io.parity.image.description="Pezcumulus, the Pezkuwi collator." \
	io.parity.image.source="https://github.com/pezkuwichain/pezkuwi-sdk/blob/${VCS_REF}/docker/dockerfiles/pezkuwi-teyrchain/pezkuwi-teyrchain-debug_unsigned_injected.Dockerfile" \
	io.parity.image.revision="${VCS_REF}" \
	io.parity.image.created="${BUILD_DATE}" \
	io.parity.image.documentation="https://github.com/pezkuwichain/pezkuwi-sdk/tree/master/pezcumulus"

# show backtraces
ENV RUST_BACKTRACE 1

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
	libssl1.1 \
	ca-certificates \
	curl && \
	# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
	# add user and link ~/.local/share/pezkuwi-teyrchain to /data
	useradd -m -u 1000 -U -s /bin/sh -d /pezkuwi-teyrchain pezkuwi-teyrchain && \
	mkdir -p /data /pezkuwi-teyrchain/.local/share && \
	chown -R pezkuwi-teyrchain:pezkuwi-teyrchain /data && \
	ln -s /data /pezkuwi-teyrchain/.local/share/pezkuwi-teyrchain && \
	mkdir -p /specs

# add pezkuwi-teyrchain binary to the docker image
COPY ./artifacts/pezkuwi-teyrchain /usr/local/bin
COPY ./pezcumulus/teyrchains/chain-specs/*.json /specs/

USER pezkuwi-teyrchain

# check if executable works in this container
RUN /usr/local/bin/pezkuwi-teyrchain --version

EXPOSE 30333 9933 9944
VOLUME ["/pezkuwi-teyrchain"]

ENTRYPOINT ["/usr/local/bin/pezkuwi-teyrchain"]
