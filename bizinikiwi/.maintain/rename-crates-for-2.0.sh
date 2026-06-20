#!/usr/bin/env bash

function rust_rename() {
    sed -i "s/$1/$2/g" `grep -Rl --include="*.rs" --include="*.stderr" "$1" *` > /dev/null
}

function cargo_rename() {
    find . -name "Cargo.toml" -exec sed -i "s/\(^\|[^\/]\)$1/\1$2/g" {} \;
}

function rename_gitlabci() {
    sed -i "s/$1/$2/g" .gitlab-ci.yml
}

function rename() {
    old=$(echo $1 | cut -f1 -d\ );
    new=$(echo $1 | cut -f2 -d\ );

    echo "Renaming $old to $new"
    # rename in Cargo.tomls
    cargo_rename $old $new
    rename_gitlabci $old $new
    # and it appears, we have the same syntax in rust files
    rust_rename $old $new

    # but generally we have the snail case syntax in rust files
    old=$(echo $old | sed s/-/_/g );
    new=$(echo $new | sed s/-/_/g );

    echo " > $old to $new"
    rust_rename $old $new
}

TO_RENAME=(
    # OLD-CRATE-NAME NEW-CRATE-NAME

    # post initial rename fixes
    "sc-application-crypto sp-application-crypto"
    "sp-transaction-pool-api sp-transaction-pool"
    "sp-transaction-pool-runtime-api sp-transaction-pool"
    "sp-core-storage sp-storage"
    "transaction-factory node-transaction-factory"
    "sp-finality-granpda sp-finality-grandpa"
    "sp-sesssion sp-session"
    "sp-tracing-pool sp-transaction-pool"
    "sc-basic-authority sc-basic-authorship"
    "sc-api sc-client-api"
    "sc-database sc-client-db"

    # PRIMITIVES
    "bizinikiwi-application-crypto sp-application-crypto"
    "bizinikiwi-authority-discovery-primitives sp-authority-discovery"
    "bizinikiwi-block-builder-runtime-api sp-block-builder"
    "bizinikiwi-consensus-aura-primitives sp-consensus-aura"
    "bizinikiwi-consensus-babe-primitives sp-consensus-babe"
    "bizinikiwi-consensus-common sp-consensus"
    "bizinikiwi-consensus-pow-primitives sp-consensus-pow"
    "bizinikiwi-primitives sp-core"
    "bizinikiwi-debug-derive sp-debug-derive"
    "bizinikiwi-primitives-storage sp-storage"
    "bizinikiwi-externalities sp-externalities"
    "bizinikiwi-finality-grandpa-primitives sp-finality-grandpa"
    "bizinikiwi-inherents sp-inherents"
    "bizinikiwi-keyring sp-keyring"
    "bizinikiwi-offchain-primitives sp-offchain"
    "bizinikiwi-panic-handler sp-panic-handler"
    "bizinikiwi-phragmen sp-npos-elections"
    "bizinikiwi-rpc-primitives sp-rpc"
    "bizinikiwi-runtime-interface sp-runtime-interface"
    "bizinikiwi-runtime-interface-proc-macro sp-runtime-interface-proc-macro"
    "bizinikiwi-runtime-interface-test-wasm sp-runtime-interface-test-wasm"
    "bizinikiwi-serializer sp-serializer"
    "bizinikiwi-session sp-session"
    "sr-api sp-api"
    "sr-api-proc-macro sp-api-proc-macro"
    "sr-api-test sp-api-test"
    "sr-arithmetic sp-arithmetic"
    "sr-arithmetic-fuzzer sp-arithmetic-fuzzer"
    "sr-io sp-io"
    "sr-primitives sp-runtime"
    "sr-sandbox sp-sandbox"
    "sr-staking-primitives sp-staking"
    "sr-std sp-std"
    "sr-version sp-version"
    "bizinikiwi-state-machine sp-state-machine"
    "bizinikiwi-transaction-pool-runtime-api sp-transaction-pool"
    "bizinikiwi-trie sp-trie"
    "bizinikiwi-wasm-interface sp-wasm-interface"

    # # CLIENT
    "bizinikiwi-client sc-client"
    "bizinikiwi-client-api sc-client-api"
    "bizinikiwi-authority-discovery sc-authority-discovery"
    "bizinikiwi-basic-authorship sc-basic-authorship"
    "bizinikiwi-block-builder sc-block-builder"
    "bizinikiwi-chain-spec sc-chain-spec"
    "bizinikiwi-chain-spec-derive sc-chain-spec-derive"
    "bizinikiwi-cli sc-cli"
    "bizinikiwi-consensus-aura sc-consensus-aura"
    "bizinikiwi-consensus-babe sc-consensus-babe"
    "bizinikiwi-consensus-pow sc-consensus-pow"
    "bizinikiwi-consensus-slots sc-consensus-slots"
    "bizinikiwi-consensus-uncles sc-consensus-uncles"
    "bizinikiwi-client-db sc-client-db"
    "bizinikiwi-executor sc-executor"
    "bizinikiwi-runtime-test sc-runtime-test"
    "bizinikiwi-finality-grandpa sc-finality-grandpa"
    "bizinikiwi-keystore sc-keystore"
    "bizinikiwi-network sc-network"
    "bizinikiwi-offchain sc-offchain"
    "bizinikiwi-peerset sc-peerset"
    "bizinikiwi-rpc-servers sc-rpc-server"
    "bizinikiwi-rpc sc-rpc"
    "bizinikiwi-service sc-service"
    "bizinikiwi-service-test sc-service-test"
    "bizinikiwi-state-db sc-state-db"
    "bizinikiwi-telemetry sc-telemetry"
    "bizinikiwi-test-primitives sp-test-primitives"
    "bizinikiwi-tracing sc-tracing"

);

for rule in "${TO_RENAME[@]}"
do
	rename "$rule";
done
