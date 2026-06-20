<div align="center">

![SDK Logo](../docs/images/PezkuwiChain_Logo_Horizontal_Pink_White.png#gh-dark-mode-only)
![SDK Logo](../docs/images/PezkuwiChain_Logo_Horizontal_Pink_Black.png#gh-light-mode-only)

<!-- markdownlint-disable-next-line MD044 -->
# `pezkuwi-sdk`

[![StackExchange](https://img.shields.io/badge/StackExchange-Community%20&%20Support-222222?logo=stackexchange)](https://pezkuwichain.io/community/)

</div>

`pezkuwi-sdk` is an umbrella crate for the [PezkuwiChain
SDK](https://github.com/pezkuwichain/pezkuwi-sdk), in the sense that it is an "umbrella" that
encompasses other components. More specifically, it re-exports all the crates that are needed by
builders.

`pezkuwi-sdk` aims to be the entry to the PezkuwiChain and Bizinikiwi ecosystem and make the SDK more
approachable—the entire development environment made available with **one dependency**. More
importantly, it guarantees the compatible combination of crate versions. So even if you know exactly
which crates to use, you may still benefit from using `pezkuwi-sdk` for painless dependency
updates.

You may have seen another umbrella crate named `pezkuwi-sdk-frame`, also known as the FRAME umbrella crate.
For clarification, while
`pezkuwi-sdk` aims to ease dependency management, `pezkuwi-sdk-frame` intends to simplify
[FRAME](https://docs.pezkuwichain.io/polkadot-protocol/glossary/#frame-framework-for-runtime-aggregation-of-modularized-entities)
pezpallet implementation, as demonstrated in the example below.

## 💻 Usage

The re-exported crates are grouped under the following feature sets.

- `node`: Anything that you need to build a node
- `runtime`: Most things that you need to build a runtime
- `runtime-full`: Also the extended runtime features that are sometimes needed

<details>
<summary>🏋️ Power User Features</summary>

- `experimental`
- `runtime-benchmarks`
- `serde`
- `tuples-96`
- `try-runtime`
- `with-tracing`

The power user features are meant to use alongside `node`, `runtime`, or `runtime-full` for extra
development support. For example, if the runtime relies on [serde](https://crates.io/crates/serde)
for serialization, and needs tracing and benchmarking for debugging and profiling, the `Cargo.toml`
may contain the following.

```toml
[dependencies]
pezkuwi-sdk = { version = "0.12.0", features = ["runtime", "serde"], default-features = false }

[features]
runtime-benchmarks = ["pezkuwi-sdk/runtime-benchmarks"]
with-tracing = ["pezkuwi-sdk/with-tracing"]
```

```shell
cargo build --features "runtime-benchmarks,with-tracing"
```

Bizinikiwi's [try-runtime](https://paritytech.github.io/try-runtime-cli/try_runtime/) is an essential
tool for testing runtime protocol upgrades locally, which can be enabled with the `try-runtime`
feature.

```toml
[dependencies]
pezkuwi-sdk = { version = "0.12.0", features = ["runtime"], default-features = false }

[features]
try-runtime = ["pezkuwi-sdk/try-runtime"]
```

```shell
cargo build --features "try-runtime"
```

In Bizinikiwi, a runtime can be seen as a tuple of various pallets. Since the number of pallets can
vary and there is no way to anticipate it, we have to generate impl-trait for tuples of different
sizes upfront, from 0-tuple to 64-tuple to be specific (64 is chosen to balance between usability
and compile time).

Seldomly, when the runtime grows to have more than 64 pallets, the trait implementations will cease
to apply, in which case the feature `tuples-96` (or even `tuples-128`) must be enabled (at the cost
of increased compile time).

```toml
[dependencies]
pezkuwi-sdk = { version = "0.12.0", features = ["runtime", "tuples-96"], default-features = false }
```

In addition to all the features mentioned earlier, each exported crate is feature-gated individually
with the name identical to the crate name, to provide fine-grained control over the dependencies.
Enabling features like `node` may pull in dependencies that you don't need. As you become more
knowledgeable about the SDK, you may consider specifying the exact crate names in the `features`
list instead to reduce build time.

</details>

---

When using `pezkuwi-sdk` to build a node, it is a good start to enable the `node` feature.

```toml
[dependencies]
pezkuwi-sdk = { version = "0.12.0", features = ["node"] }
```

For a runtime implementation, you need the `runtime` feature instead. Besides, you may want to opt
out of `std` with `default-features = false` to allow the runtime to be executed in environments
where `std` isn't available.

```toml
[dependencies]
pezkuwi-sdk = { version = "0.12.0", features = ["runtime"], default-features = false }
```

When building a runtime or writing an application pezpallet, `pezkuwi-sdk-frame` can be a handy
toolkit to start with. It gathers all the common types, traits, and functions from many different
crates so that you can import them with a one-liner.

`pezkuwi-sdk-frame` is also a part of `pezkuwi-sdk`. It is enabled by the `runtime` feature.

```rust
// It's a convention to rename it to `frame`.
use pezkuwi_sdk::pezkuwi_sdk_frame as frame;

#[frame::pezpallet(dev_mode)]
pub mod pezpallet {
    // Import declarations aren't automatically inherited.
    // Need to "re-import" to make `frame` available here.
    use super::*;
    // One-liner to import all the dependencies used here.
    use frame::prelude::*;

    pub type Balance = u128;

    #[pezpallet::pezpallet]
    pub struct Pezpallet<T>(_);

    #[pezpallet::config]
    #[pezpallet::disable_frame_system_supertrait_check]
    pub trait Config: frame_system::Config {}

    #[pezpallet::storage]
    pub type Balances<T: Config> = StorageMap<_, _, T::AccountId, Balance>;

    impl<T: Config> Pezpallet<T> {
        pub fn transfer(
            from: T::RuntimeOrigin,
            to: T::AccountId,
            amount: Balance,
        ) -> DispatchResult {
            let sender = ensure_signed(from)?;
            let sender_balance = Balances::<T>::get(&sender).ok_or("NonExistentAccount")?;
            let sender_remainder = sender_balance
                .checked_sub(amount)
                .ok_or("InsufficientBalance")?;

            Balances::<T>::mutate(to, |b| *b = Some(b.unwrap_or(0) + amount));
            Balances::<T>::insert(&sender, sender_remainder);

            Ok(())
        }
    }
}
```

For more detailed documentation and examples on `pezkuwi-sdk-frame`, see [`pezkuwi_sdk_frame`](https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_frame/index.html).

To learn more about building with the Pezkuwi SDK, you may start with these
[guides](https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_docs/guides/index.html) and
our [official docs](https://docs.pezkuwichain.io/).

## 🚀 Versioning

We do a stable release for the SDK every three months with a version schema reflecting the release
cadence, which is tracked in the [release
registry](https://github.com/paritytech/release-registry/). At the time of writing, the latest
version is `stable2412` (released in 2024 December). To avoid confusion, we will align the
versioning of `pezkuwi-sdk` with the established schema. For instance, the next stable version will
be `2503.0.0`.
