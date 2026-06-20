# PezkuwiChain Omni Benchmarking CLI

The PezkuwiChain Omni benchmarker allows to benchmark the extrinsics of any PezkuwiChain runtime. It is
meant to replace the current manual integration of the `benchmark pezpallet` into every teyrchain node.
This reduces duplicate code and makes maintenance for builders easier. The CLI is currently only
able to benchmark extrinsics. In the future it is planned to extend this to some other areas.

General FRAME runtimes could also be used with this benchmarker, as long as they don't utilize any
host functions that are not part of the PezkuwiChain host specification.

## Installation

Directly via crates.io:

```sh
cargo install frame-omni-bencher --profile=production --locked
```

from GitHub:

```sh
cargo install --git https://github.com/pezkuwichain/pezkuwi-sdk frame-omni-bencher --profile=production --locked
```

or locally from the sources:

```sh
cargo install --path bizinikiwi/utils/pezframe/omni-bencher --profile=production
```

Check the installed version and print the docs:

```sh
frame-omni-bencher --help
```

## Usage

First we need to ensure that there is a runtime available. As example we will build the zagros
runtime:

```sh
cargo build -p zagros-runtime --profile production --features runtime-benchmarks
```

Now as an example, we benchmark the `balances` pezpallet:

```sh
frame-omni-bencher v1 benchmark pezpallet \
--runtime target/release/wbuild/zagros-runtime/zagros-runtime.compact.compressed.wasm \
--pezpallet "pallet_balances" --extrinsic ""
```

The `--steps`, `--repeat`, `--heap-pages` and `--wasm-execution` arguments have sane defaults and do
not need be passed explicitly anymore.

### Generate weights (templates)

To render Rust weight files from benchmark results, pass an output path. Optionally you can pass a
custom header and a Handlebars template (defaults are provided):

```sh
frame-omni-bencher v1 benchmark pezpallet \
  --runtime target/release/wbuild/zagros-runtime/zagros-runtime.compact.compressed.wasm \
  --pezpallet "pallet_balances" --extrinsic "*" \
  --output ./weights/ \
  --header ./HEADER.rs \
  --template ./template.hbs
```

This uses the same flags as the node-integrated benchmarking CLI. The output can be a directory or a
file path; when a directory is given, a file name is generated per pezpallet/instance.

## Backwards Compatibility

The exposed pezpallet sub-command is identical as the node-integrated CLI. The only difference is that
it needs to be prefixed with a `v1` to ensure drop-in compatibility.
