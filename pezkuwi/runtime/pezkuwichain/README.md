# pezkuwichain: v2.1

pezkuwichain is a testnet runtime with no stability guarantees.

## How to build `pezkuwichain` runtime
`EpochDurationInBlocks` parameter is configurable via `pezkuwichain_EPOCH_DURATION` environment variable. To build wasm
runtime blob with customized epoch duration the following command shall be executed:
```bash
pezkuwichain_EPOCH_DURATION=10 ./pezkuwi/scripts/build-only-wasm.sh pezkuwichain-runtime /path/to/output/directory/
```

## How to run `pezkuwichain-local`

The [Pezcumulus Tutorial](https://docs.pezkuwichain.io/tutorials/v3/pezcumulus/start-relay/) details building, starting, and
testing `pezkuwichain-local` and teyrchains connecting to it.

## How to register a teyrchain on the pezkuwichain testnet

The [teyrchain registration process](https://docs.pezkuwichain.io/tutorials/v3/pezcumulus/pezkuwichain/) on the public pezkuwichain
testnet is also outlined.
