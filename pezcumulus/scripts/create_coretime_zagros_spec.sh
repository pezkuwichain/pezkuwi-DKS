#!/usr/bin/env bash

usage() {
    echo Usage:
    echo "$1 <srtool compressed runtime path>"
    echo "$2 <para_id>"
    echo "e.g.: ./pezcumulus/scripts/create_coretime_zagros_spec.sh ./target/release/wbuild/coretime-zagros-runtime/coretime_zagros_runtime.compact.compressed.wasm 1005"
    exit 1
}

if [ -z "$1" ]; then
    usage
fi

if [ -z "$2" ]; then
    usage
fi

set -e

rt_path=$1
para_id=$2

echo "Generating chain spec for runtime: $rt_path and para_id: $para_id"

binary="./target/release/pezkuwi-teyrchain"

# build the chain spec we'll manipulate
$binary build-spec --chain coretime-zagros-dev > chain-spec-plain.json

# convert runtime to hex
cat $rt_path | od -A n -v -t x1 |  tr -d ' \n' > rt-hex.txt

# replace the runtime in the spec with the given runtime and set some values to production
# Related issue for bootNodes, invulnerables, and session keys: https://github.com/paritytech/devops/issues/2725
cat chain-spec-plain.json | jq --rawfile code rt-hex.txt '.genesis.runtimeGenesis.code = ("0x" + $code)' \
    | jq '.name = "Zagros Coretime"' \
    | jq '.id = "coretime-zagros"' \
    | jq '.chainType = "Live"' \
    | jq '.bootNodes = [
          "/dns/zagros-coretime-collator-0.parity-testnet.parity.io/tcp/30333/p2p/12D3KooWP93Dzk8T7GWxyWw9jhLcz8Pksokk3R9vL2eEH337bNkT",
          "/dns/zagros-coretime-collator-1.parity-testnet.parity.io/tcp/30333/p2p/12D3KooWMh2imeAzsZKGQgm2cv6Uoep3GBYtwGfujt1bs5YfVzkH"
        ]' \
    | jq '.relay_chain = "zagros"' \
    | jq --argjson para_id $para_id '.para_id = $para_id' \
    | jq --argjson para_id $para_id '.genesis.runtimeGenesis.patch.teyrchainInfo.teyrchainId = $para_id' \
    | jq '.genesis.runtimeGenesis.patch.balances.balances = []' \
    | jq '.genesis.runtimeGenesis.patch.collatorSelection.invulnerables = [
          "5GKXTtB7RG3mLJ2kT4AkDXoxvKCFDVUdwyRmeMEbX3gBwcGi",
          "5DknBCD1h49nc8eqnm6XtHz3bMQm5hfMuGYcLenRfCmpnBJG"
        ]' \
    | jq '.genesis.runtimeGenesis.patch.session.keys = [
          [
            "5GKXTtB7RG3mLJ2kT4AkDXoxvKCFDVUdwyRmeMEbX3gBwcGi",
            "5GKXTtB7RG3mLJ2kT4AkDXoxvKCFDVUdwyRmeMEbX3gBwcGi",
            {
              "aura": "5GKXTtB7RG3mLJ2kT4AkDXoxvKCFDVUdwyRmeMEbX3gBwcGi"
            }
          ],
          [
            "5DknBCD1h49nc8eqnm6XtHz3bMQm5hfMuGYcLenRfCmpnBJG",
            "5DknBCD1h49nc8eqnm6XtHz3bMQm5hfMuGYcLenRfCmpnBJG",
            {
              "aura": "5DknBCD1h49nc8eqnm6XtHz3bMQm5hfMuGYcLenRfCmpnBJG"
            }
          ]
        ]' \
    > edited-chain-spec-plain.json

# build a raw spec
$binary build-spec --chain edited-chain-spec-plain.json --raw > chain-spec-raw.json
cp edited-chain-spec-plain.json coretime-zagros-spec.json
cp chain-spec-raw.json ./pezcumulus/teyrchains/chain-specs/coretime-zagros.json
cp chain-spec-raw.json coretime-zagros-spec-raw.json

# build genesis data
$binary export-genesis-state --chain chain-spec-raw.json > coretime-zagros-genesis-head-data

# build genesis wasm
$binary export-genesis-wasm --chain chain-spec-raw.json > coretime-zagros-wasm

# cleanup
rm -f rt-hex.txt
rm -f chain-spec-plain.json
rm -f chain-spec-raw.json
rm -f edited-chain-spec-plain.json
