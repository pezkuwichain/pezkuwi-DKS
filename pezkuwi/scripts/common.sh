#!/usr/bin/env bash

ROOT=`dirname "$0"`

# A list of directories which contain wasm projects.
SRCS=(
	"runtime/wasm"
)

DEMOS=(
	"test-teyrchains/addr test-teyrchains/halt test-teyrchains/undying"
)

# Make pushd/popd silent.

pushd () {
	command pushd "$@" > /dev/null
}

popd () {
	command popd "$@" > /dev/null
}
