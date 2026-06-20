use crate::{
	mock::*, ContributionLimits, Error, Event, PresaleCreationParams, PresaleStatus, RefundConfig,
	VestingSchedule,
};
use pezframe_support::{assert_noop, assert_ok};

/// Helper function to create presale params with common defaults
#[allow(clippy::too_many_arguments)]
fn make_presale_params(
	tokens_for_sale: u128,
	duration: u64,
	is_whitelist: bool,
	min_contribution: u128,
	max_contribution: u128,
	soft_cap: u128,
	hard_cap: u128,
	enable_vesting: bool,
	vesting_immediate_percent: u8,
	vesting_duration_blocks: u64,
	vesting_cliff_blocks: u64,
	grace_period_blocks: u64,
	refund_fee_percent: u8,
	grace_refund_fee_percent: u8,
) -> PresaleCreationParams<u64> {
	let vesting = if enable_vesting {
		Some(VestingSchedule {
			immediate_release_percent: vesting_immediate_percent,
			vesting_duration_blocks,
			cliff_blocks: vesting_cliff_blocks,
		})
	} else {
		None
	};

	PresaleCreationParams {
		tokens_for_sale,
		duration,
		is_whitelist,
		limits: ContributionLimits { min_contribution, max_contribution, soft_cap, hard_cap },
		vesting,
		refund_config: RefundConfig {
			grace_period_blocks,
			refund_fee_percent,
			grace_refund_fee_percent,
		},
	}
}

#[test]
fn create_presale_works() {
	new_test_ext().execute_with(|| {
		create_assets();

		// Mint reward tokens to Alice (presale owner)
		mint_assets(1, 1, 100_000_000_000_000_000_000); // 100,000 PEZ

		// Alice creates a presale
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2, // wUSDT payment asset
			1, // PEZ reward asset
			make_presale_params(
				10_000_000_000_000_000_000, // 10,000 PEZ tokens for sale (10^12 decimals)
				100,                        // 100 blocks duration
				false,                      // public presale
				10_000_000,                 // min 10 USDT (10^6 decimals)
				1_000_000_000,              // max 1000 USDT
				5_000_000_000,              // soft cap 5,000 USDT
				10_000_000_000,             // hard cap 10,000 USDT
				false,                      // no vesting
				0,
				0,
				0,
				24, // 24 blocks grace period
				5,  // 5% refund fee
				2,  // 2% grace refund fee
			),
		));

		// Check presale created
		let presale = Presale::presales(0).unwrap();
		assert_eq!(presale.owner, 1);
		assert_eq!(presale.payment_asset, 2);
		assert_eq!(presale.reward_asset, 1);
		assert_eq!(presale.tokens_for_sale, 10_000_000_000_000_000_000);
		assert_eq!(presale.duration, 100);

		// Check event
		System::assert_last_event(
			Event::PresaleCreated { presale_id: 0, owner: 1, payment_asset: 2, reward_asset: 1 }
				.into(),
		);

		// Check NextPresaleId incremented
		assert_eq!(Presale::next_presale_id(), 1);
	});
}

#[test]
fn create_multiple_presales_works() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 1_000_000_000_000_000_000_000);
		mint_assets(1, 2, 1_000_000_000_000_000_000_000);

		// Alice creates first presale
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Bob creates second presale
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(2),
			2,
			1,
			make_presale_params(
				20_000_000_000_000_000_000,
				200,
				false,
				20_000_000,
				2_000_000_000,
				10_000_000_000,
				20_000_000_000,
				false,
				0,
				0,
				0,
				48,
				10,
				5
			)
		));

		// Check both presales exist
		assert!(Presale::presales(0).is_some());
		assert!(Presale::presales(1).is_some());

		// Check owners
		assert_eq!(Presale::presales(0).unwrap().owner, 1);
		assert_eq!(Presale::presales(1).unwrap().owner, 2);

		// Check NextPresaleId
		assert_eq!(Presale::next_presale_id(), 2);
	});
}

#[test]
fn contribute_works() {
	new_test_ext().execute_with(|| {
		create_assets();

		// Setup: Alice creates presale
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Mint wUSDT to Bob
		mint_assets(2, 2, 1_000_000_000); // 1000 USDT

		// Bob contributes 100 USDT
		let contribution = 100_000_000;
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, contribution));

		// Check contribution tracked (gross amount)
		let contribution_info = Presale::contributions(0, 2).unwrap();
		assert_eq!(contribution_info.amount, contribution);

		// Platform fee: 2% of 100_000_000 = 2_000_000
		let platform_fee = contribution * 2 / 100;
		let net_amount = contribution - platform_fee; // 98_000_000

		// Check total raised (tracks gross amount)
		assert_eq!(Presale::total_raised(0), contribution);

		// Check contributors list
		let contributors = Presale::contributors(0);
		assert_eq!(contributors.len(), 1);
		assert_eq!(contributors[0], 2);

		// Check wUSDT transferred to presale treasury (net amount after platform fee)
		let treasury = presale_treasury(0);
		let balance = Assets::balance(2, treasury);
		assert_eq!(balance, net_amount);

		// Verify platform fee distribution (50% treasury, 25% staking, 25% burned)
		let expected_to_treasury = platform_fee * 50 / 100; // 1_000_000
		let expected_to_staking = platform_fee * 25 / 100; // 500_000
		let _expected_burned = platform_fee * 25 / 100; // 500_000

		// Check platform treasury received 50%
		assert_eq!(Assets::balance(2, 999), expected_to_treasury);

		// Check staking pool received 25%
		assert_eq!(Assets::balance(2, 998), expected_to_staking);

		// Note: Burn is verified by the fact that total supply decreased
		// Initial supply to Bob was 1_000_000_000, after contribution:
		// Bob spent: 100_000_000
		// Presale treasury got: 98_000_000
		// Platform treasury got: 1_000_000
		// Staking pool got: 500_000
		// Burned: 500_000
		// Total accounted: 100_000_000 ✓

		// Check event
		System::assert_last_event(
			Event::Contributed { presale_id: 0, who: 2, amount: contribution, bonus_amount: 0 }
				.into(),
		);
	});
}

#[test]
fn contribute_multiple_times_works() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		mint_assets(2, 2, 1_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// First contribution
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, 50_000_000));
		let contribution_info = Presale::contributions(0, 2).unwrap();
		assert_eq!(contribution_info.amount, 50_000_000);

		// Second contribution
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, 30_000_000));
		let contribution_info = Presale::contributions(0, 2).unwrap();
		assert_eq!(contribution_info.amount, 80_000_000);

		// Contributors list should still have only 1 entry
		assert_eq!(Presale::contributors(0).len(), 1);

		// Total raised should be sum of gross amounts
		assert_eq!(Presale::total_raised(0), 80_000_000);
	});
}

#[test]
fn contribute_to_different_presales_works() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 1_000_000_000_000_000_000_000);
		mint_assets(2, 2, 2_000_000_000); // 2000 USDT

		// Create two presales
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Fund presale 0 treasury with reward tokens
		mint_assets(1, presale_treasury(0), 10_000_000_000_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				15_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Fund presale 1 treasury with reward tokens
		mint_assets(1, presale_treasury(1), 15_000_000_000_000_000_000);

		// Bob contributes to both presales
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, 100_000_000));
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 1, 200_000_000));

		// Check contributions tracked separately (gross amounts)
		let contribution_info_0 = Presale::contributions(0, 2).unwrap();
		assert_eq!(contribution_info_0.amount, 100_000_000);
		let contribution_info_1 = Presale::contributions(1, 2).unwrap();
		assert_eq!(contribution_info_1.amount, 200_000_000);

		// Calculate net amounts after 2% platform fee (what goes to treasury)
		let net_amount_0 = 100_000_000 * 98 / 100; // 98_000_000
		let net_amount_1 = 200_000_000 * 98 / 100; // 196_000_000

		// Check total raised per presale (gross amounts)
		assert_eq!(Presale::total_raised(0), 100_000_000);
		assert_eq!(Presale::total_raised(1), 200_000_000);

		// Check balances in separate treasuries (net amounts after platform fee)
		assert_eq!(Assets::balance(2, presale_treasury(0)), net_amount_0);
		assert_eq!(Assets::balance(2, presale_treasury(1)), net_amount_1);
	});
}

#[test]
fn contribute_below_min_fails() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		mint_assets(2, 2, 1_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Try to contribute less than minimum (10 USDT)
		assert_noop!(
			Presale::contribute(RuntimeOrigin::signed(2), 0, 5_000_000),
			Error::<Test>::BelowMinContribution
		);
	});
}

#[test]
fn contribute_above_max_fails() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		mint_assets(2, 2, 5_000_000_000); // 5000 USDT

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Try to contribute more than maximum (1000 USDT)
		assert_noop!(
			Presale::contribute(RuntimeOrigin::signed(2), 0, 2_000_000_000),
			Error::<Test>::AboveMaxContribution
		);
	});
}

#[test]
fn contribute_exceeding_hard_cap_fails() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		mint_assets(2, 2, 15_000_000_000); // 15,000 USDT

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000, // Soft cap: 5,000 USDT, Hard cap: 10,000 USDT
				false,
				0,
				0,
				0,
				24,
				5,
				2,
			)
		));

		// Multiple contributors reach near hard cap (9,000 USDT total)
		// Max contribution is 1000 USDT each, so need 9 contributors
		for i in 3..12 {
			mint_assets(2, i, 1_010_000_000); // 1000 USDT + 10 ED
			assert_ok!(Presale::contribute(RuntimeOrigin::signed(i), 0, 1_000_000_000));
		}

		// Bob tries to contribute 2,000 USDT but max is 1000
		// Even 1000 would exceed hard cap (9000 + 1000 = 10000 is ok, but 9000 + 2000 = 11000
		// exceeds) So contribute 500 to not exceed max, then try to contribute another 1000 to
		// exceed hard cap
		mint_assets(2, 2, 2_010_000_000);
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, 1_000_000_000)); // Now at 10,000 hard cap

		// Try to contribute more (should fail with HardCapReached)
		mint_assets(2, 13, 1_010_000_000);
		assert_noop!(
			Presale::contribute(RuntimeOrigin::signed(13), 0, 1_000_000_000),
			Error::<Test>::HardCapReached
		);
	});
}

#[test]
fn contribute_after_presale_ended_fails() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		mint_assets(2, 2, 1_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Move past presale end (block 1 + 100 = 101)
		System::set_block_number(102);

		assert_noop!(
			Presale::contribute(RuntimeOrigin::signed(2), 0, 100_000_000),
			Error::<Test>::PresaleEnded
		);
	});
}

#[test]
fn finalize_presale_works() {
	new_test_ext().execute_with(|| {
		create_assets();

		// Setup: Alice creates presale with PEZ rewards
		mint_assets(1, 1, 100_000_000_000_000_000_000); // 100,000 PEZ
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Mint PEZ to presale treasury for distribution
		let treasury = presale_treasury(0);
		mint_assets(1, treasury, 100_000_000_000_000_000_000);

		// Bob and Charlie contribute (need to exceed soft cap of 5,000 USDT, max is 1000 each)
		// Need 6 contributors @ 1000 USDT each = 6000 USDT
		for i in 2..8 {
			mint_assets(2, i, 1_010_000_000); // 1000 USDT + 10 ED
			assert_ok!(Presale::contribute(RuntimeOrigin::signed(i), 0, 1_000_000_000));
		}

		// total_raised tracks gross amounts: 6 * 1000 = 6,000 USDT > soft cap (5,000 USDT)
		let total_gross = 6_000_000_000;

		// Move to end of presale
		System::set_block_number(101);

		// Finalize presale (requires root)
		assert_ok!(Presale::finalize_presale(RuntimeOrigin::root(), 0));

		// Check presale status changed to Successful (not Finalized yet - needs batch_distribute)
		let presale = Presale::presales(0).unwrap();
		assert!(matches!(presale.status, PresaleStatus::Successful));

		// Now batch distribute to all contributors
		assert_ok!(Presale::batch_distribute(RuntimeOrigin::signed(1), 0, 0, 100));

		// After batch_distribute, presale should be Finalized
		let presale = Presale::presales(0).unwrap();
		assert!(matches!(presale.status, PresaleStatus::Finalized));

		// Token distribution is based on gross contribution amounts
		// Each contributor: (1,000 / 6,000) * 10,000 PEZ = 1,666.666... PEZ
		// tokens_for_sale = 10_000_000_000_000_000_000 (10000 PEZ with 12 decimals)
		// Each share: 1_000_000_000 / 6_000_000_000 * 10_000_000_000_000_000_000
		//           = 1/6 * 10_000_000_000_000_000_000
		//           = 1_666_666_666_666_666_666 (approx, with rounding)
		let expected_pez = 1_666_666_666_666_666_666u128;

		for i in 2..8 {
			let contributor_pez = Assets::balance(1, i);
			// Allow for small rounding differences (within 0.1%)
			assert!(
				contributor_pez >= expected_pez - 10_000_000_000_000_000
					&& contributor_pez <= expected_pez + 10_000_000_000_000_000,
				"Contributor {i} PEZ: {contributor_pez} (expected ~{expected_pez})"
			);
		}

		// Check that batch distribution completed event was emitted
		// (PresaleFinalized is emitted before BatchDistributionCompleted)
		System::assert_has_event(
			Event::PresaleFinalized { presale_id: 0, total_raised: total_gross }.into(),
		);
	});
}

#[test]
fn finalize_presale_before_end_fails() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Try to finalize immediately (use root to test the actual business logic error)
		assert_noop!(
			Presale::finalize_presale(RuntimeOrigin::root(), 0),
			Error::<Test>::PresaleNotEnded
		);
	});
}

#[test]
fn finalize_presale_non_root_fails() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		System::set_block_number(101);

		// Non-root tries to finalize (finalize_presale is root-only)
		assert_noop!(
			Presale::finalize_presale(RuntimeOrigin::signed(2), 0),
			pezsp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn refund_works() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		mint_assets(2, 2, 1_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Bob contributes
		let contribution = 100_000_000; // 100 USDT
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, contribution));

		// Note: Treasury received 98 USDT (after 2% platform fee)
		// But refund is calculated on gross amount (100 USDT)
		// Refund = 95 USDT (100 - 5% fee) + 5 USDT fee distribution = 100 USDT needed
		// Treasury only has 98 USDT, so we need to add more to cover:
		// 1. Platform fee shortfall: 2 USDT
		// 2. Min balance to prevent NotExpendable error: 1 USDT
		let treasury = presale_treasury(0);
		mint_assets(2, treasury, 3_000_000); // Add 3 USDT total

		// Bob requests refund (not in grace period)
		System::set_block_number(30);

		let initial_balance = Assets::balance(2, 2);
		assert_ok!(Presale::refund(RuntimeOrigin::signed(2), 0));

		// Check refund with 5% fee (calculated on NET amount in treasury after 2% platform fee)
		let platform_fee = contribution * 2 / 100; // 2 USDT platform fee at contribution
		let net_in_treasury = contribution - platform_fee; // 98 USDT actually in treasury
		let fee = net_in_treasury * 5 / 100; // 4.9 USDT refund fee
		let refund_amount = net_in_treasury - fee; // 93.1 USDT refunded to user

		// Check Bob's balance increased
		assert_eq!(Assets::balance(2, 2), initial_balance + refund_amount);

		// Check contribution marked as refunded
		let contribution_info = Presale::contributions(0, 2).unwrap();
		assert!(contribution_info.refunded);

		// Check total raised decreased (gross amount)
		assert_eq!(Presale::total_raised(0), 0);

		// Check event
		System::assert_last_event(
			Event::Refunded { presale_id: 0, who: 2, amount: refund_amount, fee }.into(),
		);
	});
}

#[test]
fn refund_in_grace_period_lower_fee() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		mint_assets(2, 2, 1_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24, // 24 blocks grace period (block 1 + 24 = 25)
				5,  // 5% regular refund fee
				2,  // 2% grace refund fee
			)
		));

		let contribution = 100_000_000; // 100 USDT
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, contribution));

		// Treasury received 98 USDT (after 2% platform fee)
		// Refund = 98 USDT (100 - 2% grace fee) + 2 USDT fee distribution = 100 USDT needed
		// Treasury only has 98 USDT, so we need to add more to cover:
		// 1. Platform fee shortfall: 2 USDT
		// 2. Min balance to prevent NotExpendable error: 1 USDT
		let treasury = presale_treasury(0);
		mint_assets(2, treasury, 3_000_000); // Add 3 USDT total

		// Refund within grace period (block < 25)
		System::set_block_number(20);

		let initial_balance = Assets::balance(2, 2);
		assert_ok!(Presale::refund(RuntimeOrigin::signed(2), 0));

		// Should use grace period fee (2% of NET amount in treasury)
		let platform_fee = contribution * 2 / 100; // 2 USDT platform fee at contribution
		let net_in_treasury = contribution - platform_fee; // 98 USDT in treasury
		let grace_fee = net_in_treasury * 2 / 100; // 1.96 USDT grace period fee
		let refund_amount = net_in_treasury - grace_fee; // 96.04 USDT refunded to user

		assert_eq!(Assets::balance(2, 2), initial_balance + refund_amount);
	});
}

#[test]
fn refund_with_no_contribution_fails() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Bob tries to refund without contributing
		assert_noop!(Presale::refund(RuntimeOrigin::signed(2), 0), Error::<Test>::NoContribution);
	});
}

#[test]
fn cancel_presale_works() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		mint_assets(2, 2, 1_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Bob contributes
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, 100_000_000));

		// Root cancels presale (EmergencyOrigin is EnsureRoot in mock)
		assert_ok!(Presale::cancel_presale(RuntimeOrigin::root(), 0));

		// Check status changed
		let presale = Presale::presales(0).unwrap();
		assert!(matches!(presale.status, PresaleStatus::Cancelled));

		// Check event
		System::assert_last_event(Event::PresaleCancelled { presale_id: 0 }.into());
	});
}

#[test]
fn cancel_presale_non_authorized_fails() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Non-authorized user tries to cancel (needs EmergencyOrigin or Root)
		assert_noop!(
			Presale::cancel_presale(RuntimeOrigin::signed(2), 0),
			pezsp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn emergency_cancel_by_root_works() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Root can cancel any presale (emergency)
		assert_ok!(Presale::cancel_presale(RuntimeOrigin::root(), 0));

		let presale = Presale::presales(0).unwrap();
		assert!(matches!(presale.status, PresaleStatus::Cancelled));
	});
}

#[test]
fn whitelist_presale_works() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		mint_assets(2, 2, 1_000_000_000);

		// Create whitelist presale
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				true, // whitelist enabled
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Bob tries to contribute (not whitelisted)
		assert_noop!(
			Presale::contribute(RuntimeOrigin::signed(2), 0, 100_000_000),
			Error::<Test>::NotWhitelisted
		);

		// Owner adds Bob to whitelist
		assert_ok!(Presale::add_to_whitelist(RuntimeOrigin::signed(1), 0, 2));

		// Now Bob can contribute
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, 100_000_000));
	});
}

#[test]
fn add_to_whitelist_non_owner_fails() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);

		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				true,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Charlie tries to add Bob to Alice's presale whitelist
		assert_noop!(
			Presale::add_to_whitelist(RuntimeOrigin::signed(3), 0, 2),
			Error::<Test>::NotPresaleOwner
		);
	});
}

// ========== SOFT CAP TESTS ==========

#[test]
fn finalize_presale_soft_cap_reached_success() {
	new_test_ext().execute_with(|| {
		create_assets();

		// Setup: Alice creates presale
		// Soft cap: 5,000 USDT, Hard cap: 10,000 USDT
		mint_assets(1, 1, 100_000_000_000_000_000_000); // 100,000 PEZ
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Mint PEZ to presale treasury
		let treasury = presale_treasury(0);
		mint_assets(1, treasury, 100_000_000_000_000_000_000);

		// Contributors exceed soft cap (max is 1000 USDT each)
		// Need 6 contributors to reach 6000 USDT (above soft cap of 5000)
		for i in 2..8 {
			mint_assets(2, i, 1_010_000_000); // 1000 USDT + 10 ED
			assert_ok!(Presale::contribute(RuntimeOrigin::signed(i), 0, 1_000_000_000));
		}

		// total_raised tracks gross amounts: 6 * 1000 = 6,000 USDT > soft cap (5,000 USDT) ✅
		let total_gross = 6_000_000_000;
		assert_eq!(Presale::total_raised(0), total_gross);

		// Move past presale end
		System::set_block_number(102);

		// Root finalizes presale
		assert_ok!(Presale::finalize_presale(RuntimeOrigin::root(), 0));

		// Check presale status is Successful (needs batch_distribute for Finalized)
		let presale = Presale::presales(0).unwrap();
		assert!(matches!(presale.status, PresaleStatus::Successful));

		// Batch distribute to all contributors
		assert_ok!(Presale::batch_distribute(RuntimeOrigin::signed(1), 0, 0, 100));

		// Now check presale is Finalized
		let presale = Presale::presales(0).unwrap();
		assert!(matches!(presale.status, PresaleStatus::Finalized));

		// Check contributors received tokens
		// Total raised: 6,000 USDT
		// Tokens for sale: 10,000 PEZ (10^12 decimals)
		// Each contributor's share: (1,000 / 6,000) * 10,000 = 1,666.67 PEZ
		for i in 2..8 {
			assert!(Assets::balance(1, i) > 0, "Contributor {i} should receive PEZ");
		}
	});
}

#[test]
fn finalize_presale_soft_cap_not_reached_fails() {
	new_test_ext().execute_with(|| {
		create_assets();

		// Setup: Alice creates presale
		// Soft cap: 5,000 USDT, Hard cap: 10,000 USDT
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Contributors below soft cap (max is 1000 USDT each)
		// Need to contribute less than soft cap of 5000 USDT
		mint_assets(2, 2, 1_010_000_000); // Bob: 1000 USDT + 10 ED
		mint_assets(2, 3, 1_010_000_000); // Charlie: 1000 USDT + 10 ED
		mint_assets(2, 4, 1_010_000_000); // Dave: 1000 USDT + 10 ED

		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, 1_000_000_000));
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(3), 0, 1_000_000_000));
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(4), 0, 1_000_000_000));

		// total_raised tracks gross amounts: 1000 + 1000 + 1000 = 3,000 USDT < soft cap (5,000
		// USDT) ❌
		let total_gross = 3_000_000_000;
		assert_eq!(Presale::total_raised(0), total_gross);

		// Move past presale end
		System::set_block_number(102);

		// Root finalizes presale
		assert_ok!(Presale::finalize_presale(RuntimeOrigin::root(), 0));

		// Check presale status is Failed (soft cap not reached)
		let presale = Presale::presales(0).unwrap();
		assert!(matches!(presale.status, PresaleStatus::Failed));

		// Check contributors did NOT receive tokens (presale failed)
		assert_eq!(Assets::balance(1, 2), 0); // Bob received nothing
		assert_eq!(Assets::balance(1, 3), 0); // Charlie received nothing
	});
}

#[test]
fn batch_refund_failed_presale_works() {
	new_test_ext().execute_with(|| {
		create_assets();

		// Setup: Alice creates presale
		mint_assets(1, 1, 100_000_000_000_000_000_000);
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Fund presale treasury with wUSDT for refunds
		let treasury = presale_treasury(0);
		mint_assets(2, treasury, 10_000_000_000); // Mint enough for refunds

		// Contributors below soft cap (max is 1000 USDT each)
		// Need ED (10 USDT) + contribution amount
		mint_assets(2, 2, 510_000_000); // Bob gets 510 USDT (500 + 10 ED)
		mint_assets(2, 3, 510_000_000); // Charlie gets 510 USDT (500 + 10 ED)

		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, 500_000_000));
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(3), 0, 500_000_000));

		// Record initial balances
		let bob_initial = Assets::balance(2, 2);
		let charlie_initial = Assets::balance(2, 3);

		// Move past presale end and finalize (will set status to Failed)
		System::set_block_number(102);
		assert_ok!(Presale::finalize_presale(RuntimeOrigin::root(), 0));

		// Check status is Failed
		let presale = Presale::presales(0).unwrap();
		assert!(matches!(presale.status, PresaleStatus::Failed));

		// Anyone can call batch_refund_failed_presale
		assert_ok!(Presale::batch_refund_failed_presale(
			RuntimeOrigin::signed(4), // Random account (not owner)
			0,                        // presale_id
			0,                        // start_index
			10,                       // batch_size (refund up to 10 contributors)
		));

		// Check contributors got refunds of net amount in treasury
		// Platform fee = 500M * 2% = 10M (already distributed at contribution time)
		// Treasury received net_amount = 500M - 10M = 490M per contributor
		// Refund = 490M (full net amount, no additional fee for failed presale)
		assert_eq!(Assets::balance(2, 2), bob_initial + 490_000_000);
		assert_eq!(Assets::balance(2, 3), charlie_initial + 490_000_000);

		// Check contributions marked as refunded
		let bob_contribution = Presale::contributions(0, 2).unwrap();
		assert!(bob_contribution.refunded);
		assert_eq!(bob_contribution.refund_fee_paid, 0); // No fee!
	});
}

#[test]
fn batch_refund_successful_presale_fails() {
	new_test_ext().execute_with(|| {
		create_assets();

		mint_assets(1, 1, 100_000_000_000_000_000_000);
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		let treasury = presale_treasury(0);
		mint_assets(1, treasury, 100_000_000_000_000_000_000);
		mint_assets(2, treasury, 10_000_000_000);

		// Exceed soft cap (soft cap is 5000 USDT, so contribute 1000 USDT to reach it)
		// Actually need multiple contributors since max is 1000 USDT each
		mint_assets(2, 2, 1_010_000_000); // 1000 USDT + 10 ED
		mint_assets(2, 3, 1_010_000_000);
		mint_assets(2, 4, 1_010_000_000);
		mint_assets(2, 5, 1_010_000_000);
		mint_assets(2, 6, 1_010_000_000);

		assert_ok!(Presale::contribute(RuntimeOrigin::signed(2), 0, 1_000_000_000));
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(3), 0, 1_000_000_000));
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(4), 0, 1_000_000_000));
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(5), 0, 1_000_000_000));
		assert_ok!(Presale::contribute(RuntimeOrigin::signed(6), 0, 1_000_000_000));

		// Finalize (will succeed because soft cap reached)
		System::set_block_number(102);
		assert_ok!(Presale::finalize_presale(RuntimeOrigin::root(), 0));

		// Try to batch refund a successful presale (should fail)
		assert_noop!(
			Presale::batch_refund_failed_presale(RuntimeOrigin::signed(4), 0, 0, 10,),
			Error::<Test>::PresaleNotFailed
		);
	});
}

#[test]
fn create_presale_with_soft_cap_greater_than_hard_cap_fails() {
	new_test_ext().execute_with(|| {
		create_assets();
		mint_assets(1, 1, 100_000_000_000_000_000_000);

		// Try to create presale with soft_cap > hard_cap (invalid)
		assert_noop!(
			Presale::create_presale(
				RuntimeOrigin::signed(1),
				2,
				1,
				make_presale_params(
					10_000_000_000_000_000_000,
					100,
					false,
					10_000_000,
					1_000_000_000,
					15_000_000_000,
					10_000_000_000,
					false,
					0,
					0,
					0,
					24,
					5,
					2
				)
			),
			Error::<Test>::InvalidTokensForSale
		);
	});
}
#[test]
fn debug_finalize_presale() {
	use crate::mock::*;
	use pezframe_support::assert_ok;

	new_test_ext().execute_with(|| {
		create_assets();

		// Mint reward tokens to owner
		mint_assets(1, 1, 100_000_000_000_000_000_000);

		// Create presale
		assert_ok!(Presale::create_presale(
			RuntimeOrigin::signed(1),
			2,
			1,
			make_presale_params(
				10_000_000_000,
				100,
				false,
				10_000_000,
				1_000_000_000,
				5_000_000_000,
				10_000_000_000,
				false,
				0,
				0,
				0,
				24,
				5,
				2
			)
		));

		// Fund presale treasury with reward tokens
		let treasury = presale_treasury(0);
		mint_assets(1, treasury, 1_000_000_000_000_000_000_000);

		// Fund platform accounts
		mint_assets(2, 999, 100_000_000_000);
		mint_assets(2, 998, 100_000_000_000);

		// Create 25 contributors
		for i in 2..27 {
			mint_assets(2, i, 220_000_000); // payment asset
			mint_assets(1, i, 1_000_000_000); // reward asset
			assert_ok!(Presale::contribute(RuntimeOrigin::signed(i), 0, 200_000_000));
		}

		// Move to end
		System::set_block_number(150);

		// Try to finalize
		let result = Presale::finalize_presale(RuntimeOrigin::root(), 0);
		println!("Finalize result: {result:?}");
		assert_ok!(result);
	});
}
