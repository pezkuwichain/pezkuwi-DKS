//! # Pezcumulus
//!
//! Bizinikiwi provides a framework ([FRAME]) through which a blockchain node and runtime can easily
//! be created. Pezcumulus aims to extend the same approach to creation of Pezkuwi teyrchains.
//!
//! > Pezcumulus clouds are shaped sort of like dots; together they form a system that is intricate,
//! > beautiful and functional.
//!
//! ## Example: Runtime
//!
//! A Pezcumulus-based runtime is fairly similar to other [FRAME]-based runtimes. Most notably, the
//! following changes are applied to a normal FRAME-based runtime to make it a Pezcumulus-based
//! runtime:
//!
//! #### Pezcumulus Pallets
//!
//! A teyrchain runtime should use a number of pallets that are provided by Pezcumulus and
//! Bizinikiwi. Notably:
//!
//! - [`pezframe-system`](pezframe::prelude::pezframe_system), like all FRAME-based runtimes.
//! - [`pezcumulus_pezpallet_teyrchain_system`]
//! - [`teyrchain_info`]
#![doc = docify::embed!("./src/pezkuwi_sdk/pezcumulus.rs", system_pallets)]
//!
//! Given that all Pezcumulus-based runtimes use a simple Aura-based consensus mechanism, the
//! following pallets also need to be added:
//!
//! - [`pezpallet_timestamp`]
//! - [`pezpallet_aura`]
//! - [`pezcumulus_pezpallet_aura_ext`]
#![doc = docify::embed!("./src/pezkuwi_sdk/pezcumulus.rs", consensus_pallets)]
//!
//!
//! Finally, a separate macro, similar to
//! [`impl_runtime_api`](pezframe::runtime::prelude::impl_runtime_apis), which creates the default set
//! of runtime APIs, will generate the teyrchain runtime's validation runtime API, also known as
//! teyrchain validation function (PVF). Without this API, the relay chain is unable to validate
//! blocks produced by our teyrchain.
#![doc = docify::embed!("./src/pezkuwi_sdk/pezcumulus.rs", validate_block)]
//!
//! ---
//!
//! [FRAME]: crate::pezkuwi_sdk::frame_runtime

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]

#[cfg(test)]
mod tests {
	mod runtime {
		pub use pezframe::{
			deps::pezsp_consensus_aura::sr25519::AuthorityId as AuraId, prelude::*,
			runtime::prelude::*, testing_prelude::*,
		};

		#[docify::export(CR)]
		construct_runtime!(
			pub enum Runtime {
				// system-level pallets.
				System: pezframe_system,
				Timestamp: pezpallet_timestamp,
				TeyrchainSystem: pezcumulus_pezpallet_teyrchain_system,
				TeyrchainInfo: teyrchain_info,

				// teyrchain consensus support -- mandatory.
				Aura: pezpallet_aura,
				AuraExt: pezcumulus_pezpallet_aura_ext,
			}
		);

		#[docify::export]
		mod system_pallets {
			use super::*;

			#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
			impl pezframe_system::Config for Runtime {
				type Block = MockBlock<Self>;
				type OnSetCode = pezcumulus_pezpallet_teyrchain_system::TeyrchainSetCode<Self>;
			}

			impl pezcumulus_pezpallet_teyrchain_system::Config for Runtime {
				type RuntimeEvent = RuntimeEvent;
				type OnSystemEvent = ();
				type SelfParaId = teyrchain_info::Pezpallet<Runtime>;
				type OutboundXcmpMessageSource = ();
				type XcmpMessageHandler = ();
				type ReservedDmpWeight = ();
				type ReservedXcmpWeight = ();
				type CheckAssociatedRelayNumber =
					pezcumulus_pezpallet_teyrchain_system::RelayNumberMonotonicallyIncreases;
				type ConsensusHook = pezcumulus_pezpallet_aura_ext::FixedVelocityConsensusHook<
					Runtime,
					6000, // relay chain block time
					1,
					1,
				>;
				type WeightInfo = ();
				type DmpQueue = pezframe::traits::EnqueueWithOrigin<(), pezsp_core::ConstU8<0>>;
				type RelayParentOffset = ConstU32<0>;
			}

			impl teyrchain_info::Config for Runtime {}
		}

		#[docify::export]
		mod consensus_pallets {
			use super::*;

			impl pezpallet_aura::Config for Runtime {
				type AuthorityId = AuraId;
				type DisabledValidators = ();
				type MaxAuthorities = ConstU32<100_000>;
				type AllowMultipleBlocksPerSlot = ConstBool<false>;
				type SlotDuration = pezpallet_aura::MinimumPeriodTimesTwo<Self>;
			}

			#[docify::export(timestamp)]
			#[derive_impl(pezpallet_timestamp::config_preludes::TestDefaultConfig)]
			impl pezpallet_timestamp::Config for Runtime {}

			impl pezcumulus_pezpallet_aura_ext::Config for Runtime {}
		}

		#[docify::export(validate_block)]
		pezcumulus_pezpallet_teyrchain_system::register_validate_block! {
			Runtime = Runtime,
			BlockExecutor = pezcumulus_pezpallet_aura_ext::BlockExecutor::<Runtime, Executive>,
		}
	}
}
