// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Weights for pezpallet-tnpos.
//!
//! Both People runtimes bind their own generated file (`weights/pezpallet_tnpos.rs`); the
//! `()` impl below is what the mock and any runtime without generated weights get.

use pezframe_support::weights::{constants::RocksDbWeight, Weight};

pub trait WeightInfo {
	fn join() -> Weight;
	fn leave() -> Weight;
	fn evict() -> Weight;
	fn set_strata() -> Weight;
	fn report_offence() -> Weight;
	fn commit_seed() -> Weight;
	fn reveal_seed() -> Weight;
	/// `p` is the pool size: seating iterates `PoolMembers` once per seated stratum, so the
	/// cost is linear in it and a constant here would be a lie the block budget pays for.
	fn seat_committee(p: u32) -> Weight;
	/// `n` is the number of accounts reported.
	fn note_bonded(n: u32) -> Weight;
	fn set_relay_keys() -> Weight;
	fn purge_relay_keys() -> Weight;
	fn switch_stratum() -> Weight;
	/// `n` is the number of validators the session report names, scored and failed together.
	fn note_session_performance(n: u32) -> Weight;
}

/// Measured, not estimated: each figure is the larger of the two People twins' generated
/// weights from the reference box (weights runs 39841537690 and 39841543012, 2026-10-23,
/// `--steps 50 --repeat 20`), with the runtime's `DbWeight` replaced by `RocksDbWeight`.
///
/// This impl returned `Weight::zero()` for every call until 2026-08-30, which made every TNPoS
/// extrinsic free; a hand-written ceiling replaced it and the first measurement showed that
/// ceiling undercharging five of the seven calls, `seat_committee` by 107x. Neither a zero nor
/// a guess is a weight: when a benchmark changes, the generated files are re-measured and
/// these figures are copied from them.
impl WeightInfo for () {
	/// Storage: `Tnpos::Banned` (r:1 w:0)
	/// Proof: `Tnpos::Banned` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::PoolMembers` (r:1 w:1)
	/// Proof: `Tnpos::PoolMembers` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::StratumSize` (r:9 w:1)
	/// Proof: `Tnpos::StratumSize` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::Strata` (r:1 w:0)
	/// Proof: `Tnpos::Strata` (`max_values`: Some(1), `max_size`: Some(145), added: 640, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::RelayKeys` (r:1 w:0)
	/// Proof: `Tnpos::RelayKeys` (`max_values`: None, `max_size`: Some(562), added: 3037, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::BondedOnAssetHub` (r:1 w:0)
	/// Proof: `Tnpos::BondedOnAssetHub` (`max_values`: Some(1), `max_size`: Some(32770), added: 33265, mode: `MaxEncodedLen`)
	/// Storage: `Welati::ParliamentMembers` (r:1 w:0)
	/// Proof: `Welati::ParliamentMembers` (`max_values`: Some(1), `max_size`: Some(11057), added: 11552, mode: `MaxEncodedLen`)
	/// Storage: `Tiki::UserTikis` (r:1 w:0)
	/// Proof: `Tiki::UserTikis` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Tiki::TikiExpiry` (r:1 w:0)
	/// Proof: `Tiki::TikiExpiry` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::InPoolSince` (r:0 w:1)
	/// Proof: `Tnpos::InPoolSince` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	fn join() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `671`
		//  Estimated: `34255`
		// Minimum execution time: 85_653_000 picoseconds.
		Weight::from_parts(137_865_000, 0)
			.saturating_add(Weight::from_parts(0, 34255))
			.saturating_add(RocksDbWeight::get().reads(17))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
	/// Storage: `Tnpos::PoolMembers` (r:1 w:1)
	/// Proof: `Tnpos::PoolMembers` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::StratumSize` (r:1 w:1)
	/// Proof: `Tnpos::StratumSize` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::InPoolSince` (r:0 w:1)
	/// Proof: `Tnpos::InPoolSince` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	fn leave() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `234`
		//  Estimated: `3514`
		// Minimum execution time: 25_641_000 picoseconds.
		Weight::from_parts(44_901_000, 0)
			.saturating_add(Weight::from_parts(0, 3514))
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
	/// Storage: `Tnpos::PoolMembers` (r:1 w:1)
	/// Proof: `Tnpos::PoolMembers` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::Strata` (r:1 w:0)
	/// Proof: `Tnpos::Strata` (`max_values`: Some(1), `max_size`: Some(145), added: 640, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::RelayKeys` (r:1 w:0)
	/// Proof: `Tnpos::RelayKeys` (`max_values`: None, `max_size`: Some(562), added: 3037, mode: `MaxEncodedLen`)
	/// Storage: `Welati::DiwanMembers` (r:1 w:0)
	/// Proof: `Welati::DiwanMembers` (`max_values`: Some(1), `max_size`: Some(804), added: 1299, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::StratumSize` (r:1 w:1)
	/// Proof: `Tnpos::StratumSize` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::InPoolSince` (r:0 w:1)
	/// Proof: `Tnpos::InPoolSince` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	fn evict() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `421`
		//  Estimated: `4027`
		// Minimum execution time: 46_901_000 picoseconds.
		Weight::from_parts(75_332_000, 0)
			.saturating_add(Weight::from_parts(0, 4027))
			.saturating_add(RocksDbWeight::get().reads(5))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
	/// Storage: `Tnpos::PoolMembers` (r:1 w:0)
	/// Proof: `Tnpos::PoolMembers` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::EraStart` (r:1 w:0)
	/// Proof: `Tnpos::EraStart` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::CurrentEra` (r:1 w:0)
	/// Proof: `Tnpos::CurrentEra` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::SeedCommitments` (r:1 w:1)
	/// Proof: `Tnpos::SeedCommitments` (`max_values`: None, `max_size`: Some(92), added: 2567, mode: `MaxEncodedLen`)
	fn commit_seed() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `234`
		//  Estimated: `3557`
		// Minimum execution time: 23_161_000 picoseconds.
		Weight::from_parts(39_002_000, 0)
			.saturating_add(Weight::from_parts(0, 3557))
			.saturating_add(RocksDbWeight::get().reads(4))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	/// Storage: `Tnpos::EraStart` (r:1 w:0)
	/// Proof: `Tnpos::EraStart` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::CurrentEra` (r:1 w:0)
	/// Proof: `Tnpos::CurrentEra` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::SeedCommitments` (r:1 w:1)
	/// Proof: `Tnpos::SeedCommitments` (`max_values`: None, `max_size`: Some(92), added: 2567, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::NextSeed` (r:1 w:1)
	/// Proof: `Tnpos::NextSeed` (`max_values`: Some(1), `max_size`: Some(36), added: 531, mode: `MaxEncodedLen`)
	fn reveal_seed() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `314`
		//  Estimated: `3557`
		// Minimum execution time: 23_000_000 picoseconds.
		Weight::from_parts(38_522_000, 0)
			.saturating_add(Weight::from_parts(0, 3557))
			.saturating_add(RocksDbWeight::get().reads(4))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	/// Storage: `Tnpos::Strata` (r:1 w:0)
	/// Proof: `Tnpos::Strata` (`max_values`: Some(1), `max_size`: Some(145), added: 640, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::BondedOnAssetHub` (r:1 w:0)
	/// Proof: `Tnpos::BondedOnAssetHub` (`max_values`: Some(1), `max_size`: Some(32770), added: 33265, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::PoolMembers` (r:1001 w:0)
	/// Proof: `Tnpos::PoolMembers` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::RelayKeys` (r:1000 w:0)
	/// Proof: `Tnpos::RelayKeys` (`max_values`: None, `max_size`: Some(562), added: 3037, mode: `MaxEncodedLen`)
	/// Storage: `StakingScore::CachedStakingDetails` (r:224 w:0)
	/// Proof: `StakingScore::CachedStakingDetails` (`max_values`: None, `max_size`: Some(89), added: 2564, mode: `MaxEncodedLen`)
	/// Storage: `StakingScore::StakingStartBlock` (r:112 w:0)
	/// Proof: `StakingScore::StakingStartBlock` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `StakingScore::ReportedByChain` (r:112 w:0)
	/// Proof: `StakingScore::ReportedByChain` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `StakingScore::LastConfirmed` (r:112 w:0)
	/// Proof: `StakingScore::LastConfirmed` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `StakingScore::StakeFirstSeen` (r:112 w:0)
	/// Proof: `StakingScore::StakeFirstSeen` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `Welati::ParliamentMembers` (r:1 w:0)
	/// Proof: `Welati::ParliamentMembers` (`max_values`: Some(1), `max_size`: Some(11057), added: 11552, mode: `MaxEncodedLen`)
	/// Storage: `Tiki::UserTikis` (r:222 w:0)
	/// Proof: `Tiki::UserTikis` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Tiki::TikiExpiry` (r:222 w:0)
	/// Proof: `Tiki::TikiExpiry` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Welati::DiwanMembers` (r:1 w:0)
	/// Proof: `Welati::DiwanMembers` (`max_values`: Some(1), `max_size`: Some(804), added: 1299, mode: `MaxEncodedLen`)
	/// Storage: `Perwerde::PerwerdeScores` (r:111 w:0)
	/// Proof: `Perwerde::PerwerdeScores` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `Trust::TrustScores` (r:243 w:0)
	/// Proof: `Trust::TrustScores` (`max_values`: None, `max_size`: Some(64), added: 2539, mode: `MaxEncodedLen`)
	/// Storage: `Welati::AttestedRegion` (r:111 w:0)
	/// Proof: `Welati::AttestedRegion` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::SeatedSessions` (r:111 w:0)
	/// Proof: `Tnpos::SeatedSessions` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::Banned` (r:111 w:0)
	/// Proof: `Tnpos::Banned` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::SessionsObserved` (r:1 w:0)
	/// Proof: `Tnpos::SessionsObserved` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::CoFailures` (r:111 w:0)
	/// Proof: `Tnpos::CoFailures` (`max_values`: None, `max_size`: Some(81), added: 2556, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::CurrentEra` (r:1 w:1)
	/// Proof: `Tnpos::CurrentEra` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::NextSeed` (r:1 w:1)
	/// Proof: `Tnpos::NextSeed` (`max_values`: Some(1), `max_size`: Some(36), added: 531, mode: `MaxEncodedLen`)
	/// Storage: `TeyrchainSystem::UpwardDeliveryFeeFactor` (r:1 w:0)
	/// Proof: `TeyrchainSystem::UpwardDeliveryFeeFactor` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `PezkuwiXcm::SupportedVersion` (r:2 w:0)
	/// Proof: `PezkuwiXcm::SupportedVersion` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TeyrchainSystem::HostConfiguration` (r:1 w:0)
	/// Proof: `TeyrchainSystem::HostConfiguration` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `XcmpQueue::DeliveryFeeFactor` (r:1 w:0)
	/// Proof: `XcmpQueue::DeliveryFeeFactor` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `XcmpQueue::OutboundXcmpStatus` (r:1 w:0)
	/// Proof: `XcmpQueue::OutboundXcmpStatus` (`max_values`: Some(1), `max_size`: Some(2306), added: 2801, mode: `MaxEncodedLen`)
	/// Storage: `TeyrchainSystem::RelevantMessagingState` (r:1 w:0)
	/// Proof: `TeyrchainSystem::RelevantMessagingState` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tnpos::EraStart` (r:0 w:1)
	/// Proof: `Tnpos::EraStart` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::CurrentCommittee` (r:0 w:1)
	/// Proof: `Tnpos::CurrentCommittee` (`max_values`: Some(1), `max_size`: Some(2050), added: 2545, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[450, 1000]`.
	fn seat_committee(p: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `37168 + p * (219 ±0)`
		//  Estimated: `128790 + p * (3037 ±5)`
		// Minimum execution time: 37_267_804_000 picoseconds.
		Weight::from_parts(40_257_049_000, 0)
			.saturating_add(Weight::from_parts(0, 128790))
			// Standard Error: 952_378
			.saturating_add(Weight::from_parts(64_319_750, 0).saturating_mul(p.into()))
			.saturating_add(RocksDbWeight::get().reads(37))
			.saturating_add(RocksDbWeight::get().reads((4_u64).saturating_mul(p.into())))
			.saturating_add(RocksDbWeight::get().writes(4))
			.saturating_add(Weight::from_parts(0, 3037).saturating_mul(p.into()))
	}
	/// Storage: `Tnpos::BondedOnAssetHub` (r:1 w:1)
	/// Proof: `Tnpos::BondedOnAssetHub` (`max_values`: Some(1), `max_size`: Some(32770), added: 33265, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::BondReportEra` (r:1 w:1)
	/// Proof: `Tnpos::BondReportEra` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 1024]`.
	fn note_bonded(n: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `117`
		//  Estimated: `34255`
		// Minimum execution time: 26_640_000 picoseconds.
		Weight::from_parts(39_015_873, 0)
			.saturating_add(Weight::from_parts(0, 34255))
			// Standard Error: 774
			.saturating_add(Weight::from_parts(189_747, 0).saturating_mul(n.into()))
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	/// Storage: `Diwan::Members` (r:1 w:0)
	/// Proof: `Diwan::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tnpos::Strata` (r:0 w:1)
	/// Proof: `Tnpos::Strata` (`max_values`: Some(1), `max_size`: Some(145), added: 640, mode: `MaxEncodedLen`)
	fn set_strata() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4`
		//  Estimated: `1489`
		// Minimum execution time: 12_510_000 picoseconds.
		Weight::from_parts(23_261_000, 0)
			.saturating_add(Weight::from_parts(0, 1489))
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	/// Storage: `Diwan::Members` (r:1 w:0)
	/// Proof: `Diwan::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tnpos::PoolMembers` (r:1 w:1)
	/// Proof: `Tnpos::PoolMembers` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::StratumSize` (r:1 w:1)
	/// Proof: `Tnpos::StratumSize` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::CurrentCommittee` (r:1 w:1)
	/// Proof: `Tnpos::CurrentCommittee` (`max_values`: Some(1), `max_size`: Some(2050), added: 2545, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::CurrentEra` (r:1 w:0)
	/// Proof: `Tnpos::CurrentEra` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::Banned` (r:1 w:1)
	/// Proof: `Tnpos::Banned` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `Trust::TrustScores` (r:26 w:0)
	/// Proof: `Trust::TrustScores` (`max_values`: None, `max_size`: Some(64), added: 2539, mode: `MaxEncodedLen`)
	/// Storage: `TeyrchainSystem::UpwardDeliveryFeeFactor` (r:1 w:0)
	/// Proof: `TeyrchainSystem::UpwardDeliveryFeeFactor` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `PezkuwiXcm::SupportedVersion` (r:2 w:0)
	/// Proof: `PezkuwiXcm::SupportedVersion` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TeyrchainSystem::HostConfiguration` (r:1 w:0)
	/// Proof: `TeyrchainSystem::HostConfiguration` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `XcmpQueue::DeliveryFeeFactor` (r:1 w:0)
	/// Proof: `XcmpQueue::DeliveryFeeFactor` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `XcmpQueue::OutboundXcmpStatus` (r:1 w:0)
	/// Proof: `XcmpQueue::OutboundXcmpStatus` (`max_values`: Some(1), `max_size`: Some(2306), added: 2801, mode: `MaxEncodedLen`)
	/// Storage: `TeyrchainSystem::RelevantMessagingState` (r:1 w:0)
	/// Proof: `TeyrchainSystem::RelevantMessagingState` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tnpos::InPoolSince` (r:0 w:1)
	/// Proof: `Tnpos::InPoolSince` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	fn report_offence() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `9039`
		//  Estimated: `67004`
		// Minimum execution time: 516_866_000 picoseconds.
		Weight::from_parts(810_496_000, 0)
			.saturating_add(Weight::from_parts(0, 67004))
			.saturating_add(RocksDbWeight::get().reads(39))
			.saturating_add(RocksDbWeight::get().writes(5))
	}
	/// Storage: `Tnpos::BondedOnAssetHub` (r:1 w:0)
	/// Proof: `Tnpos::BondedOnAssetHub` (`max_values`: Some(1), `max_size`: Some(32770), added: 33265, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::RelayKeys` (r:1 w:1)
	/// Proof: `Tnpos::RelayKeys` (`max_values`: None, `max_size`: Some(562), added: 3037, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::RelayKeysChangedAt` (r:1 w:1)
	/// Proof: `Tnpos::RelayKeysChangedAt` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `TeyrchainSystem::UpwardDeliveryFeeFactor` (r:1 w:0)
	/// Proof: `TeyrchainSystem::UpwardDeliveryFeeFactor` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `PezkuwiXcm::SupportedVersion` (r:1 w:0)
	/// Proof: `PezkuwiXcm::SupportedVersion` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TeyrchainSystem::HostConfiguration` (r:1 w:0)
	/// Proof: `TeyrchainSystem::HostConfiguration` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TeyrchainSystem::PendingUpwardMessages` (r:1 w:1)
	/// Proof: `TeyrchainSystem::PendingUpwardMessages` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn set_relay_keys() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `33236`
		//  Estimated: `36701`
		// Minimum execution time: 598_099_000 picoseconds.
		Weight::from_parts(839_477_000, 0)
			.saturating_add(Weight::from_parts(0, 36701))
			.saturating_add(RocksDbWeight::get().reads(7))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
	/// Storage: `Tnpos::RelayKeys` (r:1 w:1)
	/// Proof: `Tnpos::RelayKeys` (`max_values`: None, `max_size`: Some(562), added: 3037, mode: `MaxEncodedLen`)
	/// Storage: `TeyrchainSystem::UpwardDeliveryFeeFactor` (r:1 w:0)
	/// Proof: `TeyrchainSystem::UpwardDeliveryFeeFactor` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `PezkuwiXcm::SupportedVersion` (r:1 w:0)
	/// Proof: `PezkuwiXcm::SupportedVersion` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TeyrchainSystem::HostConfiguration` (r:1 w:0)
	/// Proof: `TeyrchainSystem::HostConfiguration` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TeyrchainSystem::PendingUpwardMessages` (r:1 w:1)
	/// Proof: `TeyrchainSystem::PendingUpwardMessages` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tnpos::PoolMembers` (r:1 w:1)
	/// Proof: `Tnpos::PoolMembers` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::StratumSize` (r:1 w:1)
	/// Proof: `Tnpos::StratumSize` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::InPoolSince` (r:0 w:1)
	/// Proof: `Tnpos::InPoolSince` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	fn purge_relay_keys() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `334`
		//  Estimated: `4027`
		// Minimum execution time: 75_153_000 picoseconds.
		Weight::from_parts(131_394_000, 0)
			.saturating_add(Weight::from_parts(0, 4027))
			.saturating_add(RocksDbWeight::get().reads(7))
			.saturating_add(RocksDbWeight::get().writes(5))
	}
	/// Storage: `Tnpos::PoolMembers` (r:1 w:1)
	/// Proof: `Tnpos::PoolMembers` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::Strata` (r:1 w:0)
	/// Proof: `Tnpos::Strata` (`max_values`: Some(1), `max_size`: Some(145), added: 640, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::StratumSize` (r:2 w:2)
	/// Proof: `Tnpos::StratumSize` (`max_values`: None, `max_size`: Some(13), added: 2488, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::BondedOnAssetHub` (r:1 w:0)
	/// Proof: `Tnpos::BondedOnAssetHub` (`max_values`: Some(1), `max_size`: Some(32770), added: 33265, mode: `MaxEncodedLen`)
	/// Storage: `Welati::ParliamentMembers` (r:1 w:0)
	/// Proof: `Welati::ParliamentMembers` (`max_values`: Some(1), `max_size`: Some(11057), added: 11552, mode: `MaxEncodedLen`)
	/// Storage: `Tiki::UserTikis` (r:1 w:0)
	/// Proof: `Tiki::UserTikis` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Tiki::TikiExpiry` (r:1 w:0)
	/// Proof: `Tiki::TikiExpiry` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	fn switch_stratum() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `33534`
		//  Estimated: `34255`
		// Minimum execution time: 135_675_000 picoseconds.
		Weight::from_parts(229_927_000, 0)
			.saturating_add(Weight::from_parts(0, 34255))
			.saturating_add(RocksDbWeight::get().reads(8))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
	/// Storage: `Tnpos::SessionsObserved` (r:1 w:1)
	/// Proof: `Tnpos::SessionsObserved` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::SeatedSessions` (r:1000 w:1000)
	/// Proof: `Tnpos::SeatedSessions` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `Tnpos::CoFailures` (r:500 w:500)
	/// Proof: `Tnpos::CoFailures` (`max_values`: None, `max_size`: Some(81), added: 2556, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 1000]`.
	fn note_session_performance(n: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `32 + n * (44 ±0)`
		//  Estimated: `1489 + n * (2527 ±0)`
		// Minimum execution time: 14_931_000 picoseconds.
		Weight::from_parts(15_970_000, 0)
			.saturating_add(Weight::from_parts(0, 1489))
			// Standard Error: 53_559
			.saturating_add(Weight::from_parts(12_020_053, 0).saturating_mul(n.into()))
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(RocksDbWeight::get().writes(1))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2527).saturating_mul(n.into()))
	}
}
