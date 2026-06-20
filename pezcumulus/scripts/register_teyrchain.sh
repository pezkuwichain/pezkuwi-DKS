#!/usr/bin/env bash

usage() {
    echo Usage:
    echo "$0 <url> <seed> <wasm> <genesis> <teyrchain-id> <types>"
    exit 1
}

url=$1
seed=$2
wasm=$3
genesis=$4
teyrchain_id=$5
types=$6 # we can remove this once teyrchain types are included in pezkuwi-js-api

[ -z "$url" ] && usage
[ -z "$seed" ] && usage
[ -z "$wasm" ] && usage
[ -z "$types" ] && usage
[ -z "$genesis" ] && usage
[ -z "$teyrchain_id" ] && usage
if ! [ -r "$wasm" ]; then
    echo "Could not read: $wasm"
    exit 1
fi
if ! [ -r "$types" ]; then
    echo "Could not read: $types"
    exit 1
fi

if ! which pezkuwi-js-api &> /dev/null; then
    echo 'command `pezkuwi-js-api` not in PATH'
    echo "npm install -g @pezkuwi/api-cli@beta"
    exit 1
fi

set -e -x

test -f "$seed" && seed="$(cat "$seed")"

wasm=$(cat $wasm)

pezkuwi-js-api \
    --ws "${url?}" \
    --sudo \
    --seed "${seed?}" \
    --types "${types?}" \
    tx.parasSudoWrapper.sudoScheduleParaInitialize \
        "${teyrchain_id?}" \
        "{ \"genesisHead\":\"${genesis?}\", \"validationCode\":\"${wasm?}\", \"teyrchain\": true }" \
