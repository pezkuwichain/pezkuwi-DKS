#!/usr/bin/env bash

OWNER=${OWNER:-parity}
IMAGE_NAME=${IMAGE_NAME:-pezkuwi-teyrchain}

docker build --no-cache \
    --build-arg IMAGE_NAME=$IMAGE_NAME \
    -t $OWNER/$IMAGE_NAME \
    -f ./docker/dockerfiles/pezkuwi-teyrchain/pezkuwi-teyrchain_injected.Dockerfile \
    . && docker images
