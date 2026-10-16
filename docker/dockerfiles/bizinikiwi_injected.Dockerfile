# Based on the Substrate image Dockerfile by Parity Technologies (Apache-2.0).
FROM docker.io/library/ubuntu:24.04@sha256:534baea6a22c03a63003dbc8dbe78fe34bc0d7e595d9a9dc9834884ff530eb55

# metadata
ARG VCS_REF
ARG BUILD_DATE
ARG IMAGE_NAME

LABEL org.opencontainers.image.title="${IMAGE_NAME}" \
	org.opencontainers.image.description="Bizinikiwi development node (kitchensink runtime)." \
	org.opencontainers.image.vendor="pezkuwichain" \
	org.opencontainers.image.source="https://github.com/pezkuwichain/pezkuwi-DKS" \
	org.opencontainers.image.revision="${VCS_REF}" \
	org.opencontainers.image.created="${BUILD_DATE}"

# show backtraces
ENV RUST_BACKTRACE=1

# The binary is built on the runner, so the base has to carry a glibc at least as new as the
# runner's (Ubuntu 24.04). The `--version` step below fails the build if it does not.
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get upgrade -y && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
		ca-certificates \
		curl && \
# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
# add user (24.04 ships an `ubuntu` user on uid 1000)
	userdel -r ubuntu 2>/dev/null; \
	useradd -m -u 1000 -U -s /bin/sh -d /bizinikiwi bizinikiwi

# add bizinikiwi binary to docker image
COPY ./artifacts/bizinikiwi /usr/local/bin

USER bizinikiwi

# check if executable works in this container
RUN /usr/local/bin/bizinikiwi --version

EXPOSE 30333 9933 9944
VOLUME ["/bizinikiwi"]

ENTRYPOINT ["/usr/local/bin/bizinikiwi"]
