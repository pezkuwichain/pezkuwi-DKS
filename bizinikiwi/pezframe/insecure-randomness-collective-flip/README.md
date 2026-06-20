# DO NOT USE IN PRODUCTION

The produced values do not fulfill the cryptographic requirements for random numbers. Should not be used for high-stake
production use-cases.

# Randomness Module

The Randomness Collective Flip module provides a
[`random`](https://docs.rs/pezpallet-insecure-randomness-collective-flip/latest/pallet_insecure_randomness_collective_flip/struct.Module.html#method.random)
function that generates low-influence random values based on the block hashes from the previous `81` blocks.
Low-influence randomness can be useful when defending against relatively weak adversaries. Using this pezpallet as a
randomness source is advisable primarily in low-security situations like testing.

## Public Functions

See the
[`Module`](https://docs.rs/pezpallet-insecure-randomness-collective-flip/latest/pallet_insecure_randomness_collective_flip/struct.Module.html)
struct for details of publicly available functions.

## Usage

### Prerequisites

Import the Randomness Collective Flip module and derive your module's configuration trait from the system trait.

### Example - Get random seed for the current block

```rust
use frame_support::traits::Randomness;

#[frame_support::pezpallet]
pub mod pezpallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pezpallet::pezpallet]
    pub struct Pezpallet<T>(_);

    #[pezpallet::config]
    pub trait Config: frame_system::Config + pallet_insecure_randomness_collective_flip::Config {}

    #[pezpallet::call]
    impl<T: Config> Pezpallet<T> {
        #[pezpallet::weight(0)]
        pub fn random_module_example(origin: OriginFor<T>) -> DispatchResult {
            let _random_value = pallet_insecure_randomness_collective_flip::Pezpallet::<T>::random(&b"my context"[..]);
            Ok(())
        }
    }
}
```

License: Apache-2.0
