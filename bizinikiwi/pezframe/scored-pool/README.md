# Scored Pool Module

The module maintains a scored membership pool. Each entity in the
pool can be attributed a `Score`. From this pool a set `Members`
is constructed. This set contains the `MemberCount` highest
scoring entities. Unscored entities are never part of `Members`.

If an entity wants to be part of the pool a deposit is required.
The deposit is returned when the entity withdraws or when it
is removed by an entity with the appropriate authority.

Every `Period` blocks the set of `Members` is refreshed from the
highest scoring members in the pool and, no matter if changes
occurred, `T::MembershipChanged::set_members_sorted` is invoked.
On first load `T::MembershipInitialized::initialize_members` is
invoked with the initial `Members` set.

It is possible to withdraw candidacy/resign your membership at any
time. If an entity is currently a member, this results in removal
from the `Pool` and `Members`; the entity is immediately replaced
by the next highest scoring candidate in the pool, if available.

- [`scored_pool::Trait`](https://docs.rs/pezpallet-scored-pool/latest/pallet_scored_pool/trait.Config.html)
- [`Call`](https://docs.rs/pezpallet-scored-pool/latest/pallet_scored_pool/enum.Call.html)
- [`Module`](https://docs.rs/pezpallet-scored-pool/latest/pallet_scored_pool/struct.Module.html)

## Interface

### Public Functions

- `submit_candidacy` - Submit candidacy to become a member. Requires a deposit.
- `withdraw_candidacy` - Withdraw candidacy. Deposit is returned.
- `score` - Attribute a quantitative score to an entity.
- `kick` - Remove an entity from the pool and members. Deposit is returned.
- `change_member_count` - Changes the amount of candidates taken into `Members`.

## Usage

```rust
use pallet_scored_pool::{self as scored_pool};

#[frame_support::pezpallet]
pub mod pezpallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pezpallet::pezpallet]
    pub struct Pezpallet<T>(_);

    #[pezpallet::config]
    pub trait Config: frame_system::Config + scored_pool::Config {}

    #[pezpallet::call]
    impl<T: Config> Pezpallet<T> {
        #[pezpallet::weight(0)]
        pub fn candidate(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let _ = <scored_pool::Pezpallet<T>>::submit_candidacy(
                T::RuntimeOrigin::from(Some(who.clone()).into())
            );
            Ok(())
        }
    }
}
```

## Dependencies

This module depends on the [System module](https://docs.rs/pezframe-system/latest/frame_system/).

License: Apache-2.0
