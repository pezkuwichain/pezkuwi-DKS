# Timestamp Module

The Timestamp module provides functionality to get and set the on-chain time.

- [`timestamp::Config`](https://docs.rs/pezpallet-timestamp/latest/pallet_timestamp/pezpallet/trait.Config.html)
- [`Call`](https://docs.rs/pezpallet-timestamp/latest/pallet_timestamp/pezpallet/enum.Call.html)
- [`Pezpallet`](https://docs.rs/pezpallet-timestamp/latest/pallet_timestamp/pezpallet/struct.Pezpallet.html)

## Overview

The Timestamp module allows the validators to set and validate a timestamp with each block.

It uses inherents for timestamp data, which is provided by the block author and validated/verified
by other validators. The timestamp can be set only once per block and must be set each block.
There could be a constraint on how much time must pass before setting the new timestamp.

**NOTE:** The Timestamp module is the recommended way to query the on-chain time instead of using
an approach based on block numbers. The block number based time measurement can cause issues
because of cumulative calculation errors and hence should be avoided.

## Interface

### Dispatchable Functions

- `set` - Sets the current time.

### Public functions

- `get` - Gets the current time for the current block. If this function is called prior to
setting the timestamp, it will return the timestamp of the previous block.

### Config Getters

- `MinimumPeriod` - Gets the minimum (and advised) period between blocks for the chain.

## Usage

The following example shows how to use the Timestamp module in your custom module to query the current timestamp.

### Prerequisites

Import the Timestamp module into your custom module and derive the module configuration
trait from the timestamp trait.

### Get current timestamp

```rust
use pallet_timestamp::{self as timestamp};

#[frame_support::pezpallet]
pub mod pezpallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pezpallet::pezpallet]
    pub struct Pezpallet<T>(_);

    #[pezpallet::config]
    pub trait Config: frame_system::Config + timestamp::Config {}

    #[pezpallet::call]
    impl<T: Config> Pezpallet<T> {
        #[pezpallet::weight(0)]
        pub fn get_time(origin: OriginFor<T>) -> DispatchResult {
            let _sender = ensure_signed(origin)?;
            let _now = <timestamp::Pezpallet<T>>::get();
            Ok(())
        }
    }
}
```

### Example from the FRAME

The [Session module](https://github.com/pezkuwichain/pezkuwi-sdk/blob/master/bizinikiwi/pezframe/session/src/lib.rs) uses
the Timestamp module for session management.

## Related Modules

- [Session](https://docs.rs/pezpallet-session/latest/pallet_session/)

License: Apache-2.0
