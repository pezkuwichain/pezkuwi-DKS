FROM docker.io/paritytech/base-bin

# metadata
ARG VCS_REF
ARG BUILD_DATE
ARG IMAGE_NAME
# That can be a single one or a comma separated list
ARG BINARY=pezkuwi

LABEL io.parity.image.authors="devops-team@parity.io" \
	io.parity.image.vendor="Parity Technologies" \
	io.parity.image.title="parity/pezkuwi" \
	io.parity.image.description="Pezkuwi: a platform for web3. This is the official Parity image with an injected binary." \
	io.parity.image.source="https://github.com/pezkuwichain/pezkuwi-sdk/blob/${VCS_REF}/docker/dockerfiles/pezkuwi/pezkuwi_injected.Dockerfile" \
	io.parity.image.revision="${VCS_REF}" \
	io.parity.image.created="${BUILD_DATE}" \
	io.parity.image.documentation="https://github.com/pezkuwichain/pezkuwi-sdk/"

# show backtraces
ENV RUST_BACKTRACE 1

USER root
WORKDIR /app

# add pezkuwi and pezkuwi-*-worker binaries to the docker image
COPY bin/* /usr/local/bin/
COPY entrypoint.sh .


RUN chmod -R a+rx "/usr/local/bin"; \
		mkdir -p /data /pezkuwi/.local/share && \
		chown -R parity:parity /data && \
		ln -s /data /pezkuwi/.local/share/pezkuwi

USER parity

# check if executable works in this container
RUN /usr/local/bin/pezkuwi --version
RUN /usr/local/bin/pezkuwi-prepare-worker --version
RUN /usr/local/bin/pezkuwi-execute-worker --version


EXPOSE 30333 9933 9944 9615
VOLUME ["/pezkuwi"]

ENV BINARY=${BINARY}

# ENTRYPOINT
ENTRYPOINT ["/app/entrypoint.sh"]

# We call the help by default
CMD ["--help"]
