#!/usr/bin/env bash

usage() {
    echo Usage:
    echo "$1 <srtool compressed runtime path>"
    echo "$2 <para_id>"
    echo "e.g.: ./pezcumulus/scripts/create_people_pezkuwichain_spec.sh ./target/release/wbuild/people-pezkuwichain-runtime/people_pezkuwichain_runtime.compact.compressed.wasm 1004"
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
$binary build-spec --chain people-pezkuwichain-local > chain-spec-plain.json

# convert runtime to hex
cat $rt_path | od -A n -v -t x1 |  tr -d ' \n' > rt-hex.txt

# replace the runtime in the spec with the given runtime and set some values to production
# Boot nodes, invulnerables, and session keys from https://github.com/paritytech/devops/issues/2847
#
# Note: This is a testnet runtime. Each invulnerable's Aura key is also used as its AccountId. This
# is not recommended in value-bearing networks.
cat chain-spec-plain.json | jq --rawfile code rt-hex.txt '.genesis.runtimeGenesis.code = ("0x" + $code)' \
    | jq '.name = "Pezkuwichain People"' \
    | jq '.id = "people-pezkuwichain"' \
    | jq '.chainType = "Live"' \
    | jq '.bootNodes = [
		"/dns/pezkuwichain-people-collator-node-0.parity-testnet.parity.io/tcp/30333/p2p/12D3KooWDZg5jMYhKXTu6RU491V5sxsFnP4oaEmZJEUfcRkYzps5",
		"/dns/pezkuwichain-people-collator-node-0.parity-testnet.parity.io/tcp/443/wss/p2p/12D3KooWDZg5jMYhKXTu6RU491V5sxsFnP4oaEmZJEUfcRkYzps5",
		"/dns/pezkuwichain-people-collator-node-1.parity-testnet.parity.io/tcp/30333/p2p/12D3KooWGGR5i6qQqfo7iDNp7vjDRKPWuDk53idGV6nFLwS12X5H",
		"/dns/pezkuwichain-people-collator-node-1.parity-testnet.parity.io/tcp/443/wss/p2p/12D3KooWGGR5i6qQqfo7iDNp7vjDRKPWuDk53idGV6nFLwS12X5H",
		"/dns/pezkuwichain-people-collator-node-2.parity-testnet.parity.io/tcp/30333/p2p/12D3KooWBvA9BmBfrsVMcAcqVXGYFCpMTvkSk2igNXpmoareYbeT",
		"/dns/pezkuwichain-people-collator-node-2.parity-testnet.parity.io/tcp/443/wss/p2p/12D3KooWBvA9BmBfrsVMcAcqVXGYFCpMTvkSk2igNXpmoareYbeT",
		"/dns/pezkuwichain-people-collator-node-3.parity-testnet.parity.io/tcp/30333/p2p/12D3KooWQ7Q9jLcJTPXy7KEp5hSZ8YMY9pHx9CnQVz3T8TKQ81UG",
		"/dns/pezkuwichain-people-collator-node-3.parity-testnet.parity.io/tcp/443/wss/p2p/12D3KooWQ7Q9jLcJTPXy7KEp5hSZ8YMY9pHx9CnQVz3T8TKQ81UG"
	]' \
    | jq '.relay_chain = "pezkuwichain"' \
    | jq --argjson para_id $para_id '.para_id = $para_id' \
    | jq --argjson para_id $para_id '.genesis.runtimeGenesis.patch.teyrchainInfo.teyrchainId = $para_id' \
    | jq '.genesis.runtimeGenesis.patch.balances.balances = []' \
    | jq '.genesis.runtimeGenesis.patch.collatorSelection.invulnerables = [
		"5Gnjmw1iuF2kV4PecFgetJed7B8quBKfLiRM99ELcXvFH9Vn",
		"5FLZRxyeRPhG69zo4ZPqCJSYboSKaRBUjBvQc1nkuWoBpZ5P",
		"5DNnmPH2MT6SXpfqbJZbTz4eERmuZegssfxc4ysL8PWrHaNN",
		"5DkKcSP5MboNMpXScW1CyRqaktKMXH8QLP4Mn49TwS5vhL6k"
	]' \
    | jq '.genesis.runtimeGenesis.patch.session.keys = [
            [
                "5Gnjmw1iuF2kV4PecFgetJed7B8quBKfLiRM99ELcXvFH9Vn",
                "5Gnjmw1iuF2kV4PecFgetJed7B8quBKfLiRM99ELcXvFH9Vn",
                    {
                        "aura": "5Gnjmw1iuF2kV4PecFgetJed7B8quBKfLiRM99ELcXvFH9Vn"
                    }
            ],
            [
                "5FLZRxyeRPhG69zo4ZPqCJSYboSKaRBUjBvQc1nkuWoBpZ5P",
                "5FLZRxyeRPhG69zo4ZPqCJSYboSKaRBUjBvQc1nkuWoBpZ5P",
                    {
                        "aura": "5FLZRxyeRPhG69zo4ZPqCJSYboSKaRBUjBvQc1nkuWoBpZ5P"
                    }
            ],
            [
                "5DNnmPH2MT6SXpfqbJZbTz4eERmuZegssfxc4ysL8PWrHaNN",
                "5DNnmPH2MT6SXpfqbJZbTz4eERmuZegssfxc4ysL8PWrHaNN",
                    {
                        "aura": "5DNnmPH2MT6SXpfqbJZbTz4eERmuZegssfxc4ysL8PWrHaNN"
                    }
            ],
            [
                "5DkKcSP5MboNMpXScW1CyRqaktKMXH8QLP4Mn49TwS5vhL6k",
                "5DkKcSP5MboNMpXScW1CyRqaktKMXH8QLP4Mn49TwS5vhL6k",
                    {
                        "aura": "5DkKcSP5MboNMpXScW1CyRqaktKMXH8QLP4Mn49TwS5vhL6k"
                    }
            ]
        ]' \
    > edited-chain-spec-plain.json

# build a raw spec
$binary build-spec --chain edited-chain-spec-plain.json --raw > chain-spec-raw.json
cp edited-chain-spec-plain.json people-pezkuwichain-spec.json
cp chain-spec-raw.json ./pezcumulus/teyrchains/chain-specs/people-pezkuwichain.json
cp chain-spec-raw.json people-pezkuwichain-spec-raw.json

# build genesis data
$binary export-genesis-state --chain chain-spec-raw.json > people-pezkuwichain-genesis-head-data

# build genesis wasm
$binary export-genesis-wasm --chain chain-spec-raw.json > people-pezkuwichain-wasm
