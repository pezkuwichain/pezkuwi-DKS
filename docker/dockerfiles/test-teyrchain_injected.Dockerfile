FROM docker.io/library/ubuntu:20.04

# metadata
ARG VCS_REF
ARG BUILD_DATE
ARG IMAGE_NAME

LABEL io.parity.image.authors="devops-team@parity.io" \
	io.parity.image.vendor="Parity Technologies" \
	io.parity.image.title="${IMAGE_NAME}" \
	io.parity.image.description="Test teyrchain for Zombienet" \
	io.parity.image.source="https://github.com/pezkuwichain/pezkuwi-sdk/blob/${VCS_REF}/docker/dockerfiles/test-teyrchain_injected.Dockerfile" \
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
	# add user and link ~/.local/share/test-teyrchain to /data
	useradd -m -u 10000 -U -s /bin/sh -d /test-teyrchain test-teyrchain && \
	mkdir -p /data /test-teyrchain/.local/share && \
	chown -R test-teyrchain:test-teyrchain /data && \
	ln -s /data /test-teyrchain/.local/share/test-teyrchain && \
	mkdir -p /specs

# add test-teyrchain binary to the docker image
COPY ./artifacts/test-teyrchain /usr/local/bin
COPY ./pezcumulus/teyrchains/chain-specs/*.json /specs/

USER test-teyrchain

# check if executable works in this container
RUN /usr/local/bin/test-teyrchain --version

EXPOSE 30333 9933 9944
VOLUME ["/test-teyrchain"]

ENTRYPOINT ["/usr/local/bin/test-teyrchain"]
