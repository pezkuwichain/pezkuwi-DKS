#!/usr/bin/env bash

set -e

cumulus_repo=$(cd "$(dirname "$0")" && git rev-parse --show-toplevel)
pezkuwi_repo=$(dirname "$cumulus_repo")/pezkuwi
if [ ! -d "$pezkuwi_repo/.git" ]; then
    echo "please clone pezkuwi in parallel to this repo:"
    echo "  (cd .. && git clone git@github.com:paritytech/pezkuwi.git)"
    exit 1
fi

if [ -z "$BRANCH" ]; then
    BRANCH=pezcumulus-branch
fi

cd "$pezkuwi_repo"
git fetch
git checkout "$BRANCH"
time docker build \
    -f ./docker/Dockerfile \
    --build-arg PROFILE=release \
    -t pezkuwi:"$BRANCH" .
