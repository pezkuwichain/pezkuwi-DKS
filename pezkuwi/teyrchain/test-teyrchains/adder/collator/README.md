# How to run this collator

First, build PezkuwiChain:

```sh
cargo build --release
```

Then start two validators that will run for the relay chain:

```sh
cargo run --release -- -d alice --chain pezkuwichain-local --validator --alice --port 50551
cargo run --release -- -d bob --chain pezkuwichain-local --validator --bob --port 50552
```

Next start the collator that will collate for the adder teyrchain:

```sh
cargo run --release -p test-teyrchain-adder-collator -- --tmp --chain pezkuwichain-local --port 50553
```

The last step is to register the teyrchain using `pezkuwi-js`. The teyrchain id is
100. The genesis state and the validation code are printed at startup by the collator.

To do this automatically, run `scripts/adder-collator.sh`.
