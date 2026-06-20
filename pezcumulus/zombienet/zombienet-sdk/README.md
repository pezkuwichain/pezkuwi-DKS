# How to run locally

As a prerequisite, the `test-teyrchain` and `pezkuwi` binaries need to be installed or available under `$PATH`.

The following commands need to be run from the repository root:
```
# install test-teyrchain
cargo install --path ./pezcumulus/test/service --locked
# install pezkuwi
cargo install --path ./pezkuwi --locked
```

The following command launches the tests:

```
ZOMBIE_PROVIDER=native cargo test --release -p pezcumulus-zombienet-sdk-tests --features zombie-ci
```

You can also just use `run.sh` that setups everything for you and runs the tests.

In addition, you can specify a base directory with `ZOMBIENET_SDK_BASE_DIR=/my/dir/of/choice`. All chain files and logs
will be placed in that directory.
