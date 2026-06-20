<div align="center">

![SDK Logo](../../../docs/images/PezkuwiChain_Logo_Horizontal_Pink_White.png#gh-dark-mode-only)
![SDK Logo](../../../docs/images/PezkuwiChain_Logo_Horizontal_Pink_Black.png#gh-light-mode-only)

<!-- markdownlint-disable-next-line MD044 -->

# `pezkuwi-sdk-frame`

[![StackExchange](https://img.shields.io/badge/StackExchange-Community%20&%20Support-222222?logo=stackexchange)](https://pezkuwichain.io/community/)

</div>

`pezkuwi-sdk-frame` is an umbrella crate for the
[FRAME](https://docs.pezkuwichain.io/polkadot-protocol/glossary/#frame-framework-for-runtime-aggregation-of-modularized-entities)
framework. It simplifies building FRAME pallets and runtimes by re-exporting all the necessary components for pezpallet development.

Outside the Pezkuwi SDK, `pezkuwi-sdk-frame` should be imported through the main Pezkuwi SDK [`umbrella crate`](../../../umbrella/src/lib.rs).

## 💻 Usage

The main intended use of this crate is through **Preludes**, which re-export most of the components needed for pezpallet
development. The available preludes are:

- `prelude`: main prelude for pezpallet development, containing essential types and traits
- `testing_prelude`: testing utilities and helpers for writing pezpallet tests
- `runtime::prelude`: runtime-specific components for building blockchain runtimes
- `benchmarking::prelude`: benchmarking components for performance testing
- `weights_prelude`: components for the auto-generated `weight.rs` files

If you need specific dependencies that aren't included in the preludes, you can use the `deps` module to access all
FRAME and Bizinikiwi dependencies directly. However, we strongly recommend checking the preludes and domain-specific
modules first, as they provide a more organized and maintainable way to access these dependencies.

### 📚 Documentation

For more detailed documentation and examples, see [`pezkuwi_sdk_frame`](https://docs.pezkuwichain.io/sdk/master/polkadot_sdk_frame/index.html).
