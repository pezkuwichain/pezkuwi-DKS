# Maintaining `pezkuwi-sdk-frame`

This document provides guidelines for maintaining the `pezkuwi-sdk-frame` umbrella crate.

## Code Organization

The crate is organized into three main sections:

1. **Preludes**: Re-exports of commonly used components
   - `prelude`
   - `testing_prelude`
   - `runtime::prelude`
   - `benchmarking::prelude`
   - `weights_prelude`

2. **Domain-specific modules**: Specialized functionality
   - `traits`
   - `hashing`
   - `arithmetic`
   - `derive`
   - ...

3. **Direct dependencies**: Access to all FRAME and Bizinikiwi dependencies via `deps`

## Design Principles

1. **Prelude Usage**:
   - Preludes should be extensive and comprehensive
   - The primary goal is for the crate to be used with preludes
   - Domain-specific modules serve as a backup for organization
   - Add items to preludes if they are likely to be used across numerous pallets

2. **Top-level Exports**:
   - The only non-module, non-prelude item exported from the top level is the `pezpallet` macro
   - This enables the `#[frame::pezpallet] mod pezpallet { .. }` syntax

3. **Module Organization**:
   - Create domain-specific modules (e.g., `hashing`) and add them to preludes when appropriate
   - Keep items out of preludes if they are specific to a single pezpallet, even if they're in `pezframe-support`/`pezsp-runtime`
   - Currency-related traits are kept separate to encourage deliberate choice between alternatives
   - `runtime::apis` should expose all common runtime APIs needed by FRAME-based runtimes

## Maintenance Guidelines

1. **Adding New Re-exports**:
   - Place them in the appropriate prelude or domain-specific module, creating a new ad-hoc module if necessary
   - Ensure they are properly documented
   - Update the README.md and the documentation if necessary

2. **Modifying Existing Re-exports**:
   - Maintain backward compatibility
   - Update documentation to reflect changes
   - Consider the impact on dependent crates, since this may affect multiple pallets that already rely on the FRAME
   umbrella crate

3. **Testing**:
   - Ensure all examples in documentation still work
   - Check that all dependent crates still compile

4. **Documentation**:
   - Keep the [`README.md`](./README.md) up to date
   - Document any breaking changes and possibly reach out to the community
   - Update inline documentation for new or modified components
