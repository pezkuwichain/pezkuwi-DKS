#!/usr/bin/env bash

# Sample call:
# $0 /path/to/folder_with_binary
# This script replace the former dedicated Dockerfile
# and shows how to use the generic binary_injected.dockerfile

PROJECT_ROOT=`git rev-parse --show-toplevel`

export BINARY=pezkuwi,pezkuwi-execute-worker,pezkuwi-prepare-worker
export DOCKERFILE="docker/dockerfiles/pezkuwi/pezkuwi_injected_debian.Dockerfile"
export PEZKUWI_DEB=true
export VERSION=$1

$PROJECT_ROOT/docker/scripts/build-injected.sh
