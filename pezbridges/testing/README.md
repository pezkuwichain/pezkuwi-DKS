# Bridges Tests for Local pezkuwichain <> zagros Bridge

This folder contains [zombienet](https://github.com/pezkuwichain/zombienet-sdk/) based integration tests for both
onchain and offchain bridges code.

Prerequisites for running the tests locally:

- download latest [zombienet release](https://github.com/pezkuwichain/zombienet-sdk/releases) and place it at
`~/local_bridge_testing/bin/zombienet`;

- build PezkuwiChain binary by running `cargo build -p pezkuwi --release  --features fast-runtime` command in the
  [`pezkuwi-sdk`](https://github.com/pezkuwichain/pezkuwi-sdk) repository clone;

- build PezkuwiChain Teyrchain binary by running `cargo build -p pezkuwi-teyrchain-bin --release` command in the
  [`pezkuwi-sdk`](https://github.com/pezkuwichain/pezkuwi-sdk) repository clone;

- ensure that you have [`node`](https://nodejs.org/en) installed. Additionally, we'll need the globally installed
  `pezkuwi/api-cli` package. Use `yarn global add @pezkuwi/api-cli` to install it.

- build Bizinikiwi relay by running `cargo build -p bizinikiwi-relay --release` command in the
  [`pezkuwichain/pezkuwi-sdk`](https://github.com/pezkuwichain/pezkuwi-sdk/tree/main/bridges) repository clone;

- copy the `bizinikiwi-relay` binary, built in the previous step, to `~/local_bridge_testing/bin/bizinikiwi-relay`;

On Mac, you'll also need to do the following:

- Install an updated version of bash by installing homebrew and running `brew install bash`;

- Install jq with `brew install jq`;

After that, any test can be run using the `run-test.sh` command.
Example: `./run-test.sh 0001-asset-transfer`

Hopefully, it'll show the
"All tests have completed successfully" message in the end. Otherwise, it'll print paths to zombienet
process logs, which, in turn, may be used to track locations of all spinned relay and teyrchain nodes.
