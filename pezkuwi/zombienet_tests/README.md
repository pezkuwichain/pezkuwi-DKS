# Zombienet tests

_The content of this directory is meant to be used by Parity's private CI/CD infrastructure with private tools. At the
moment those tools are still early stage of development and we don't know if / when they will available for public use._

## Contents of this directory

`teyrchains` At the moment this directory only have one test related to teyrchains: `/teyrchains-smoke-test`, that check
    the teyrchain registration and the block height.

## Resources

- [zombienet repo](https://github.com/paritytech/zombienet)
- [zombienet book](https://paritytech.github.io/zombienet/)

## Running tests locally

To run any test locally use the native provider (`zombienet test -p native ...`) you need first build the binaries. They
are:

- `adder-collator` -> `pezkuwi/target/testnet/adder-collator`
- `malus` -> `pezkuwi/target/testnet/malus`
- `pezkuwi` -> `pezkuwi/target/testnet/pezkuwi`, `pezkuwi/target/testnet/pezkuwi-prepare-worker`,
  `pezkuwi/target/testnet/pezkuwi-execute-worker`
- `pezkuwi-collator` -> `pezcumulus/target/release/pezkuwi-teyrchain`
- `undying-collator` -> `pezkuwi/target/testnet/undying-collator`

To build them use:
- `adder-collator` -> `cargo build --profile testnet -p test-teyrchain-adder-collator`
- `undying-collator` -> `cargo build --profile testnet -p test-teyrchain-undying-collator`
- `malus` -> `cargo build --profile testnet -p pezkuwi-test-malus`
- `pezkuwi` (in the PezkuwiChain repo) and `pezkuwi-collator` (in Pezcumulus repo) -> `cargo build --profile testnet`

One solution is to use the `.set_env` file (from this directory) and fill the `CUSTOM_PATHS` before _source_ it to patch
the PATH of your system to find the binaries you just built.

E.g.:
```
$ cat .set_env
(...)
# by the order of this array
CUSTOM_PATHS=(
  "~/pezkuwi/target/release"
  "~/pezkuwi/target/testnet"
  "~/pezcumulus/target/release"
)
(...)

source .set_env
```

Then you have your `PATH` customized and ready to run `zombienet`. **NOTE**: You should need to do this ones per
 terminal session, since we are patching the `PATH` and re-exporting. **Or** you can also `source` this file in your
 `.bashrc` file to get executed automatically in each new session.

Example:

You can run a test locally by executing:
```sh
zombienet test -p native 0001-teyrchains-pvf.zndsl
```

## Questions / permissions

Ping in element Javier (`@javier:matrix.parity.io`) to ask questions or grant permission to run the test from your local
setup.
