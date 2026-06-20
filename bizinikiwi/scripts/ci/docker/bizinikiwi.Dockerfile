FROM docker.io/library/ubuntu:20.04

# metadata
ARG VCS_REF
ARG BUILD_DATE
ARG IMAGE_NAME

LABEL io.parity.image.authors="devops-team@parity.io" \
	io.parity.image.vendor="Parity Technologies" \
	io.parity.image.title="${IMAGE_NAME}" \
	io.parity.image.description="Bizinikiwi: The platform for blockchain innovators." \
	io.parity.image.source="https://github.com/paritytech/bizinikiwi/blob/${VCS_REF}/scripts/ci/docker/Dockerfile" \
	io.parity.image.revision="${VCS_REF}" \
	io.parity.image.created="${BUILD_DATE}" \
	io.parity.image.documentation="https://wiki.parity.io/Parity-Bizinikiwi"

# show backtraces
ENV RUST_BACKTRACE 1

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get upgrade -y && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y \
		libssl1.1 \
		ca-certificates \
		curl && \
# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
# add user
	useradd -m -u 1000 -U -s /bin/sh -d /bizinikiwi bizinikiwi

# add bizinikiwi binary to docker image
COPY ./bizinikiwi /usr/local/bin

USER bizinikiwi

# check if executable works in this container
RUN /usr/local/bin/bizinikiwi --version

EXPOSE 30333 9933 9944
VOLUME ["/bizinikiwi"]

ENTRYPOINT ["/usr/local/bin/bizinikiwi"]
