# Aura Module

- [`aura::Config`](https://docs.rs/pezpallet-aura/latest/pallet_aura/pezpallet/trait.Config.html)
- [`Pezpallet`](https://docs.rs/pezpallet-aura/latest/pallet_aura/pezpallet/struct.Pezpallet.html)

## Overview

The Aura module extends Aura consensus by managing offline reporting.

## Interface

### Public Functions

- `slot_duration` - Determine the Aura slot-duration based on the Timestamp module configuration.

## Related Modules

- [Timestamp](https://docs.rs/pezpallet-timestamp/latest/pallet_timestamp/): The Timestamp module is used in Aura to track
consensus rounds (via `slots`).

## References

If you're interested in hacking on this module, it is useful to understand the interaction with
`bizinikiwi/primitives/inherents/src/lib.rs` and, specifically, the required implementation of
[`ProvideInherent`](https://docs.rs/pezsp-inherents/latest/sp_inherents/trait.ProvideInherent.html) and
[`ProvideInherentData`](https://docs.rs/pezsp-inherents/latest/sp_inherents/trait.ProvideInherentData.html) to create and
check inherents.

License: Apache-2.0
