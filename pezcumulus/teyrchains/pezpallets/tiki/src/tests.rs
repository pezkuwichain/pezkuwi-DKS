// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{
	mock::*, Error, Event, RoleAssignmentType, Tiki as TikiEnum, TikiProvider, TikiScoreProvider,
};
use pezframe_support::{assert_noop, assert_ok};
use pezsp_runtime::DispatchError;

type TikiPallet = crate::Pezpallet<Test>;

// === Basic NFT and Role Tests ===

#[test]
fn honorary_citizenship_works() {
	new_test_ext().execute_with(|| {
		let user_account = 2;

		// Initially there should be no citizenship NFT
		assert_eq!(TikiPallet::citizen_nft(user_account), None);
		assert!(TikiPallet::user_tikis(user_account).is_empty());
		assert!(!TikiPallet::is_citizen(&user_account));

		// Mint citizenship NFT
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user_account));

		// Verify the NFT was minted and the Welati role was added
		assert!(TikiPallet::citizen_nft(user_account).is_some());
		assert!(TikiPallet::is_citizen(&user_account));
		let user_tikis = TikiPallet::user_tikis(user_account);
		assert!(user_tikis.contains(&TikiEnum::Welati));
		assert!(TikiPallet::has_tiki(&user_account, &TikiEnum::Welati));

		// Verify the event was emitted correctly
		System::assert_has_event(
			Event::CitizenNftMinted {
				who: user_account,
				nft_id: TikiPallet::citizen_nft(user_account).unwrap(),
			}
			.into(),
		);
	});
}

#[test]
fn grant_appointed_role_works() {
	new_test_ext().execute_with(|| {
		let user_account = 2;
		let tiki_to_grant = TikiEnum::PisporêEwlehiyaSîber; // Appointed role

		// First mint the citizenship NFT
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user_account));

		// Grant the Tiki
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user_account, tiki_to_grant));

		// Verify the user's roles
		let user_tikis = TikiPallet::user_tikis(user_account);
		assert!(user_tikis.contains(&TikiEnum::Welati)); // Automatically added
		assert!(user_tikis.contains(&tiki_to_grant)); // Manually added
		assert!(TikiPallet::has_tiki(&user_account, &tiki_to_grant));

		// Verify the event was emitted correctly
		System::assert_has_event(
			Event::TikiGranted { who: user_account, tiki: tiki_to_grant }.into(),
		);
	});
}

#[test]
fn cannot_grant_elected_role_through_admin() {
	new_test_ext().execute_with(|| {
		let user_account = 2;
		let elected_role = TikiEnum::Parlementer; // Elected role

		// Mint citizenship NFT
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user_account));

		// Attempt to grant the elected role via admin - should fail
		assert_noop!(
			TikiPallet::grant_tiki(RuntimeOrigin::root(), user_account, elected_role),
			Error::<Test>::InvalidRoleAssignmentMethod
		);
	});
}

// === KYC and Identity Tests ===

#[test]
fn apply_for_citizenship_works_with_kyc() {
	new_test_ext().execute_with(|| {
		let user_account = 2;

		// Simple KYC test - skip the Identity setup, just test force mint
		// Test directly via force mint (KYC bypass)
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user_account));

		// Verify the NFT was minted
		assert!(TikiPallet::citizen_nft(user_account).is_some());
		assert!(TikiPallet::user_tikis(user_account).contains(&TikiEnum::Welati));
		assert!(TikiPallet::is_citizen(&user_account));
	});
}

#[test]
fn apply_for_citizenship_fails_without_kyc() {
	new_test_ext().execute_with(|| {
		let user_account = 2;

		// Apply for citizenship without KYC
		assert_noop!(
			TikiPallet::apply_for_citizenship(RuntimeOrigin::signed(user_account)),
			Error::<Test>::KycNotCompleted
		);
	});
}

#[test]
fn auto_grant_citizenship_simplified() {
	new_test_ext().execute_with(|| {
		let user = 2;

		// Since the Identity setup is complex, just test that the function runs
		// When called without KYC it should not error (it should simply do nothing)
		assert_ok!(TikiPallet::auto_grant_citizenship(&user));

		// No NFT should be minted because there is no KYC
		assert!(TikiPallet::citizen_nft(user).is_none());
	});
}

// === Role Assignment Types Tests ===

#[test]
fn role_assignment_types_work_correctly() {
	new_test_ext().execute_with(|| {
		// Test role types
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Welati),
			RoleAssignmentType::Automatic
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::PisporêEwlehiyaSîber),
			RoleAssignmentType::Appointed
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Parlementer),
			RoleAssignmentType::Elected
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Serok),
			RoleAssignmentType::Elected
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Axa),
			RoleAssignmentType::Earned
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::SerokêKomele),
			RoleAssignmentType::Earned
		);

		// Test can_grant_role_type
		assert!(TikiPallet::can_grant_role_type(
			&TikiEnum::PisporêEwlehiyaSîber,
			&RoleAssignmentType::Appointed
		));
		assert!(TikiPallet::can_grant_role_type(
			&TikiEnum::Parlementer,
			&RoleAssignmentType::Elected
		));
		assert!(TikiPallet::can_grant_role_type(&TikiEnum::Axa, &RoleAssignmentType::Earned));

		// Cross-type assignment should fail
		assert!(!TikiPallet::can_grant_role_type(
			&TikiEnum::PisporêEwlehiyaSîber,
			&RoleAssignmentType::Elected
		));
		assert!(!TikiPallet::can_grant_role_type(
			&TikiEnum::Parlementer,
			&RoleAssignmentType::Appointed
		));
		assert!(!TikiPallet::can_grant_role_type(&TikiEnum::Serok, &RoleAssignmentType::Appointed));
	});
}

#[test]
fn grant_earned_role_works() {
	new_test_ext().execute_with(|| {
		let user_account = 2;
		let earned_role = TikiEnum::Axa; // Earned role

		// Mint citizenship NFT
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user_account));

		// Grant the earned role
		assert_ok!(TikiPallet::grant_earned_role(RuntimeOrigin::root(), user_account, earned_role));

		// Verify the role was added
		assert!(TikiPallet::user_tikis(user_account).contains(&earned_role));
		assert!(TikiPallet::has_tiki(&user_account, &earned_role));
	});
}

#[test]
fn grant_elected_role_works() {
	new_test_ext().execute_with(|| {
		let user_account = 2;
		let elected_role = TikiEnum::Parlementer; // Elected role

		// Mint citizenship NFT
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user_account));

		// Grant the elected role (will be called by pezpallet-voting)
		assert_ok!(TikiPallet::grant_elected_role(
			RuntimeOrigin::root(),
			user_account,
			elected_role
		));

		// Verify the role was added
		assert!(TikiPallet::user_tikis(user_account).contains(&elected_role));
		assert!(TikiPallet::has_tiki(&user_account, &elected_role));
	});
}

// === Unique Roles Tests ===

#[test]
fn unique_roles_work_correctly() {
	new_test_ext().execute_with(|| {
		let user1 = 2;
		let user2 = 3;
		let unique_role = TikiEnum::Serok; // Unique role

		// Mint NFT for both users
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user1));
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user2));

		// Grant the unique role to the first user (as an elected role)
		assert_ok!(TikiPallet::grant_elected_role(RuntimeOrigin::root(), user1, unique_role));

		// Attempt to grant the same role to the second user
		assert_noop!(
			TikiPallet::grant_elected_role(RuntimeOrigin::root(), user2, unique_role),
			Error::<Test>::RoleAlreadyTaken
		);

		// Verify it was recorded correctly in TikiHolder
		assert_eq!(TikiPallet::tiki_holder(unique_role), Some(user1));
	});
}

#[test]
fn unique_role_identification_works() {
	new_test_ext().execute_with(|| {
		// Unique roles
		assert!(TikiPallet::is_unique_role(&TikiEnum::Serok));
		assert!(TikiPallet::is_unique_role(&TikiEnum::SerokiMeclise));
		assert!(TikiPallet::is_unique_role(&TikiEnum::Xezinedar));
		assert!(TikiPallet::is_unique_role(&TikiEnum::Balyoz));

		// Non-unique roles
		assert!(!TikiPallet::is_unique_role(&TikiEnum::PisporêEwlehiyaSîber));
		assert!(!TikiPallet::is_unique_role(&TikiEnum::Parlementer));
		assert!(!TikiPallet::is_unique_role(&TikiEnum::Welati));
		assert!(!TikiPallet::is_unique_role(&TikiEnum::Mamoste));
	});
}

#[test]
fn revoke_tiki_works() {
	new_test_ext().execute_with(|| {
		let user_account = 2;
		let tiki_to_revoke = TikiEnum::PisporêEwlehiyaSîber;

		// Mint NFT and grant the role
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user_account));
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user_account, tiki_to_revoke));

		// Verify the role was added
		assert!(TikiPallet::user_tikis(user_account).contains(&tiki_to_revoke));

		// Revoke the role
		assert_ok!(TikiPallet::revoke_tiki(RuntimeOrigin::root(), user_account, tiki_to_revoke));

		// Verify the role was removed
		assert!(!TikiPallet::user_tikis(user_account).contains(&tiki_to_revoke));
		assert!(!TikiPallet::has_tiki(&user_account, &tiki_to_revoke));
		// Verify the Welati role is still present
		assert!(TikiPallet::user_tikis(user_account).contains(&TikiEnum::Welati));

		// Verify the event
		System::assert_has_event(
			Event::TikiRevoked { who: user_account, tiki: tiki_to_revoke }.into(),
		);
	});
}

#[test]
fn cannot_revoke_hemwelati_role() {
	new_test_ext().execute_with(|| {
		let user_account = 2;

		// Mint NFT
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user_account));

		// Citizenship is not an office anyone can take away. It used to answer
		// `RoleNotAssigned` here, which said the wrong thing -- the role is assigned, it is
		// simply not revocable, and removing citizenship is `identity-kyc`'s to do.
		assert_noop!(
			TikiPallet::revoke_tiki(RuntimeOrigin::root(), user_account, TikiEnum::Welati),
			Error::<Test>::RoleNotRevocable
		);
	});
}

#[test]
fn revoke_unique_role_clears_holder() {
	new_test_ext().execute_with(|| {
		let user = 2;
		let unique_role = TikiEnum::Serok; // Unique role

		// Mint NFT and grant the unique role
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));
		assert_ok!(TikiPallet::grant_elected_role(RuntimeOrigin::root(), user, unique_role));

		// Verify it is recorded in TikiHolder
		assert_eq!(TikiPallet::tiki_holder(unique_role), Some(user));

		// Revoke the role
		assert_ok!(TikiPallet::revoke_tiki(RuntimeOrigin::root(), user, unique_role));

		// Verify it was cleared from TikiHolder
		assert_eq!(TikiPallet::tiki_holder(unique_role), None);
		assert!(!TikiPallet::user_tikis(user).contains(&unique_role));
	});
}

// === Scoring System Tests ===

#[test]
fn tiki_scoring_works_correctly() {
	new_test_ext().execute_with(|| {
		let user = 2;

		// Mint NFT (Welati is added automatically - 10 points)
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));
		assert_eq!(TikiPallet::get_tiki_score(&user), 10);

		// An office. It carries a bonus in the table and contributes none of it: holding
		// office earns the holder nothing.
		assert_ok!(TikiPallet::grant_elected_role(RuntimeOrigin::root(), user, TikiEnum::Serok));
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Serok), 200);
		assert_eq!(TikiPallet::get_tiki_score(&user), 10);

		// Add another role
		assert_ok!(TikiPallet::grant_earned_role(RuntimeOrigin::root(), user, TikiEnum::Axa)); // 250 points

		// 10 citizenship + 250 landholder. The presidency adds nothing.
		assert_eq!(TikiPallet::get_tiki_score(&user), 260);
	});
}

#[test]
fn scoring_system_comprehensive() {
	new_test_ext().execute_with(|| {
		// Test individual scores - according to Anayasa v5.0
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Axa), 250);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::RêveberêProjeyê), 250);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Serok), 200);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::ModeratorêCivakê), 200);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::EndameDiwane), 175);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::SerokiMeclise), 150);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Dadger), 150);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Wezir), 100);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Dozger), 120);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::SerokêKomele), 100);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Parlementer), 100);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Xezinedar), 100);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::PisporêEwlehiyaSîber), 100);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Bazargan), 60); // Newly added
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Mela), 50);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Feqî), 50);
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Welati), 10);

		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Pêseng), 80);
	});
}

#[test]
fn scoring_updates_after_role_changes() {
	new_test_ext().execute_with(|| {
		let user = 2;

		// Mint NFT
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));

		// Add two roles
		assert_ok!(TikiPallet::grant_tiki(
			RuntimeOrigin::root(),
			user,
			TikiEnum::PisporêEwlehiyaSîber
		)); // 100 points
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user, TikiEnum::Dadger)); // 150 points

		// Total: 10 + 100 + 150 = 260
		assert_eq!(TikiPallet::get_tiki_score(&user), 260);

		// Revoke one role
		assert_ok!(TikiPallet::revoke_tiki(
			RuntimeOrigin::root(),
			user,
			TikiEnum::PisporêEwlehiyaSîber
		));

		// The score should be updated: 10 + 150 = 160
		assert_eq!(TikiPallet::get_tiki_score(&user), 160);
	});
}

// === Multiple Users and Isolation Tests ===

#[test]
fn multiple_users_work_independently() {
	new_test_ext().execute_with(|| {
		let user1 = 2;
		let user2 = 3;

		// Mint NFT for both users
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user1));
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user2));

		// Grant different roles
		assert_ok!(TikiPallet::grant_earned_role(RuntimeOrigin::root(), user1, TikiEnum::Axa)); // 250 points
		assert_ok!(TikiPallet::grant_tiki(
			RuntimeOrigin::root(),
			user2,
			TikiEnum::PisporêEwlehiyaSîber
		)); // 100 points

		// Verify the scores
		assert_eq!(TikiPallet::get_tiki_score(&user1), 260); // 10 + 250
		assert_eq!(TikiPallet::get_tiki_score(&user2), 110); // 10 + 100

		// Verify the roles are distributed correctly
		assert!(TikiPallet::user_tikis(user1).contains(&TikiEnum::Axa));
		assert!(!TikiPallet::user_tikis(user1).contains(&TikiEnum::PisporêEwlehiyaSîber));

		assert!(TikiPallet::user_tikis(user2).contains(&TikiEnum::PisporêEwlehiyaSîber));
		assert!(!TikiPallet::user_tikis(user2).contains(&TikiEnum::Axa));

		// TikiProvider trait tests
		assert!(TikiPallet::has_tiki(&user1, &TikiEnum::Axa));
		assert!(!TikiPallet::has_tiki(&user1, &TikiEnum::PisporêEwlehiyaSîber));
		assert_eq!(TikiPallet::get_user_tikis(&user1).len(), 2); // Welati + Axa
	});
}

// === Edge Cases and Error Handling ===

#[test]
fn cannot_grant_role_without_citizen_nft() {
	new_test_ext().execute_with(|| {
		let user_account = 2;

		// Attempt to grant a role without an NFT
		assert_noop!(
			TikiPallet::grant_tiki(
				RuntimeOrigin::root(),
				user_account,
				TikiEnum::PisporêEwlehiyaSîber
			),
			Error::<Test>::CitizenNftNotFound
		);
	});
}

#[test]
fn nft_id_increments_correctly() {
	new_test_ext().execute_with(|| {
		let users = [2, 3, 4];

		for (i, user) in users.iter().enumerate() {
			assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), *user));
			assert_eq!(TikiPallet::citizen_nft(*user), Some(i as u32));
		}

		// Verify the next ID increments correctly
		assert_eq!(TikiPallet::next_item_id(), users.len() as u32);
	});
}

#[test]
fn duplicate_roles_not_allowed() {
	new_test_ext().execute_with(|| {
		let user = 2;
		let role = TikiEnum::Mamoste;

		// Mint NFT and grant the role
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));
		assert_ok!(TikiPallet::grant_earned_role(RuntimeOrigin::root(), user, role));

		// Attempt to grant the same role again
		assert_noop!(
			TikiPallet::grant_earned_role(RuntimeOrigin::root(), user, role),
			Error::<Test>::UserAlreadyHasRole
		);
	});
}

#[test]
fn citizen_nft_already_exists_error() {
	new_test_ext().execute_with(|| {
		let user = 2;

		// Mint the first NFT
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));

		// Attempt to mint an NFT again for the same user
		assert_noop!(
			TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user),
			Error::<Test>::CitizenNftAlreadyExists
		);
	});
}

#[test]
fn cannot_revoke_role_user_does_not_have() {
	new_test_ext().execute_with(|| {
		let user = 2;
		let role = TikiEnum::PisporêEwlehiyaSîber;

		// Mint NFT but do not grant a role
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));

		// Attempt to revoke a role the user does not have
		assert_noop!(
			TikiPallet::revoke_tiki(RuntimeOrigin::root(), user, role),
			Error::<Test>::RoleNotAssigned
		);
	});
}

// === NFT Transfer Protection Tests ===

#[test]
fn a_citizen_nft_cannot_be_transferred() {
	// The two tests this replaces called `check_transfer_permission` and asserted it returned
	// an error. It did -- but nothing called it, so what they proved was that a function
	// nobody used said the right thing. This tries the transfer itself.
	new_test_ext().execute_with(|| {
		let user1 = 2;
		let user2 = 3;

		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user1));
		let item_id = TikiPallet::citizen_nft(user1).expect("no citizen NFT was minted");

		assert!(
			pezpallet_nfts::Pezpallet::<Test>::transfer(
				RuntimeOrigin::signed(user1),
				TikiCollectionId::get(),
				item_id,
				user2,
			)
			.is_err(),
			"a citizen NFT was transferable"
		);

		assert_eq!(TikiPallet::citizen_nft(user1), Some(item_id));
		assert_eq!(TikiPallet::citizen_nft(user2), None);
	});
}

// === Trait Integration Tests ===

#[test]
fn tiki_provider_trait_works() {
	new_test_ext().execute_with(|| {
		let user = 2;

		// Mint NFT
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));
		assert_ok!(TikiPallet::grant_tiki(
			RuntimeOrigin::root(),
			user,
			TikiEnum::PisporêEwlehiyaSîber
		));

		// Test the TikiProvider trait functions
		assert!(TikiPallet::is_citizen(&user));
		assert!(TikiPallet::has_tiki(&user, &TikiEnum::Welati));
		assert!(TikiPallet::has_tiki(&user, &TikiEnum::PisporêEwlehiyaSîber));
		assert!(!TikiPallet::has_tiki(&user, &TikiEnum::Serok));

		let user_tikis = TikiPallet::get_user_tikis(&user);
		assert_eq!(user_tikis.len(), 2);
		assert!(user_tikis.contains(&TikiEnum::Welati));
		assert!(user_tikis.contains(&TikiEnum::PisporêEwlehiyaSîber));
	});
}

#[test]
fn complex_multi_role_scenario() {
	new_test_ext().execute_with(|| {
		let user = 2;

		// Mint NFT
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));

		// Add roles of various types
		assert_ok!(TikiPallet::grant_tiki(
			RuntimeOrigin::root(),
			user,
			TikiEnum::PisporêEwlehiyaSîber
		)); // Appointed
		assert_ok!(TikiPallet::grant_earned_role(RuntimeOrigin::root(), user, TikiEnum::Mamoste)); // Earned
		assert_ok!(TikiPallet::grant_elected_role(
			RuntimeOrigin::root(),
			user,
			TikiEnum::Parlementer
		)); // Elected

		// Verify all roles were added
		let user_tikis = TikiPallet::user_tikis(user);
		assert!(user_tikis.contains(&TikiEnum::Welati)); // 10 points
		assert!(user_tikis.contains(&TikiEnum::PisporêEwlehiyaSîber)); // 100 points
		assert!(user_tikis.contains(&TikiEnum::Mamoste)); // 70 points
		assert!(user_tikis.contains(&TikiEnum::Parlementer)); // a seat: 0 towards standing

		// 10 citizenship + 100 specialist + 70 teacher. The seat contributes nothing.
		assert_eq!(TikiPallet::get_tiki_score(&user), 180);

		// Revoke one role and verify the score is updated
		assert_ok!(TikiPallet::revoke_tiki(
			RuntimeOrigin::root(),
			user,
			TikiEnum::PisporêEwlehiyaSîber
		));
		assert_eq!(TikiPallet::get_tiki_score(&user), 80); // 180 - 100 specialist
	});
}

#[test]
fn role_assignment_type_logic_comprehensive() {
	new_test_ext().execute_with(|| {
		// Automatic roles
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Welati),
			RoleAssignmentType::Automatic
		);

		// Elected roles
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Parlementer),
			RoleAssignmentType::Elected
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::SerokiMeclise),
			RoleAssignmentType::Elected
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Serok),
			RoleAssignmentType::Elected
		);

		// Earned roles (social roles + some expert roles)
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Axa),
			RoleAssignmentType::Earned
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::SerokêKomele),
			RoleAssignmentType::Earned
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::ModeratorêCivakê),
			RoleAssignmentType::Earned
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Mamoste),
			RoleAssignmentType::Earned
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Rewsenbîr),
			RoleAssignmentType::Earned
		);

		// Appointed roles (officer roles - default)
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::PisporêEwlehiyaSîber),
			RoleAssignmentType::Appointed
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Dadger),
			RoleAssignmentType::Appointed
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Mela),
			RoleAssignmentType::Appointed
		);
		assert_eq!(
			TikiPallet::get_role_assignment_type(&TikiEnum::Bazargan),
			RoleAssignmentType::Appointed
		);
	});
}

// === Performance and Stress Tests ===

#[test]
fn stress_test_multiple_users_roles() {
	new_test_ext().execute_with(|| {
		let users = vec![2, 3, 4, 5];

		// Mint NFT for all users
		for user in &users {
			assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), *user));
		}

		// Grant different role combinations to each user

		// User 2: High-level elected roles
		assert_ok!(TikiPallet::grant_elected_role(RuntimeOrigin::root(), 2, TikiEnum::Serok)); // Unique
		assert_ok!(TikiPallet::grant_tiki(
			RuntimeOrigin::root(),
			2,
			TikiEnum::PisporêEwlehiyaSîber
		));

		// User 3: Technical roles
		assert_ok!(TikiPallet::grant_earned_role(RuntimeOrigin::root(), 3, TikiEnum::Mamoste));
		assert_ok!(TikiPallet::grant_tiki(
			RuntimeOrigin::root(),
			3,
			TikiEnum::PisporêEwlehiyaSîber
		));

		// User 4: Democratic roles
		assert_ok!(TikiPallet::grant_elected_role(RuntimeOrigin::root(), 4, TikiEnum::Parlementer));
		assert_ok!(TikiPallet::grant_elected_role(
			RuntimeOrigin::root(),
			4,
			TikiEnum::SerokiMeclise
		)); // Unique

		// User 5: Mixed roles
		assert_ok!(TikiPallet::grant_earned_role(RuntimeOrigin::root(), 5, TikiEnum::Axa));
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), 5, TikiEnum::Dadger));

		// Verify the scores
		// Offices contribute nothing. What is left is citizenship and what each person earned.
		assert_eq!(TikiPallet::get_tiki_score(&2), 110); // 10 welati + 100 security specialist
												   //                                                 Serok 200 is an office: nothing
		assert_eq!(TikiPallet::get_tiki_score(&3), 180); // 10 + 70 teacher + 100 specialist
		assert_eq!(TikiPallet::get_tiki_score(&4), 10); //  10; both seats are offices
		assert_eq!(TikiPallet::get_tiki_score(&5), 410); // 10 + 250 landholder + 150 judge

		// Verify the unique roles are assigned correctly
		assert_eq!(TikiPallet::tiki_holder(TikiEnum::Serok), Some(2));
		assert_eq!(TikiPallet::tiki_holder(TikiEnum::SerokiMeclise), Some(4));

		// Verify the total citizen count
		let mut citizen_count = 0;
		for user in &users {
			if TikiPallet::is_citizen(user) {
				citizen_count += 1;
			}
		}
		assert_eq!(citizen_count, 4);
	});
}

#[test]
fn maximum_roles_per_user_limit() {
	new_test_ext().execute_with(|| {
		let user = 2;

		// Mint NFT
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));

		// For testing purposes add only a few roles (to avoid exceeding the metadata length limit)
		let roles_to_add = vec![
			TikiEnum::PisporêEwlehiyaSîber,
			TikiEnum::Dadger,
			TikiEnum::Dozger,
			TikiEnum::Noter,
			TikiEnum::Bacgir,
			TikiEnum::Berdevk,
		];

		// Add the roles
		for role in roles_to_add {
			if TikiPallet::can_grant_role_type(&role, &RoleAssignmentType::Appointed) {
				assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user, role));
			}
		}

		// Verify the user holds many roles
		let final_tikis = TikiPallet::user_tikis(user);
		assert!(final_tikis.len() >= 5); // Should be at least 5 roles (Welati + 4 or more others)
		assert!(final_tikis.len() <= 100); // Should not exceed the max limit

		// Verify the total score is reasonable
		assert!(TikiPallet::get_tiki_score(&user) > 200);
	});
}

// ============================================================================
// apply_for_citizenship Edge Cases (4 tests)
// ============================================================================

#[test]
fn apply_for_citizenship_twice_same_user() {
	new_test_ext().execute_with(|| {
		let user = 5;

		// First application - use force_mint to bypass KYC
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));

		let first_score = TikiPallet::get_tiki_score(&user);
		assert_eq!(first_score, 10);

		// Attempt to mint a second time (should fail - the NFT already exists)
		assert_noop!(
			TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user),
			Error::<Test>::CitizenNftAlreadyExists
		);

		let second_score = TikiPallet::get_tiki_score(&user);
		assert_eq!(second_score, 10); // The score should not change
	});
}

#[test]
fn apply_for_citizenship_adds_hemwelati() {
	new_test_ext().execute_with(|| {
		let user = 6;

		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));

		// The Welati role is present
		let tikis = TikiPallet::user_tikis(user);
		assert!(tikis.contains(&TikiEnum::Welati));
	});
}

#[test]
fn apply_for_citizenship_initial_score() {
	new_test_ext().execute_with(|| {
		let user = 7;

		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));

		// Welati score is 10
		let score = TikiPallet::get_tiki_score(&user);
		assert_eq!(score, 10);
	});
}

#[test]
fn apply_for_citizenship_multiple_users_independent() {
	new_test_ext().execute_with(|| {
		let users = vec![8, 9, 10, 11, 12];

		for user in &users {
			assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), *user));
		}

		// All should have a score of 10
		for user in &users {
			assert_eq!(TikiPallet::get_tiki_score(user), 10);
		}
	});
}

// ============================================================================
// revoke_tiki Tests (3 tests)
// ============================================================================

#[test]
fn revoke_tiki_reduces_score() {
	new_test_ext().execute_with(|| {
		let user = 13;

		// Mint NFT and add a role
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user, TikiEnum::Dadger));

		let initial_score = TikiPallet::get_tiki_score(&user);
		assert!(initial_score > 10);

		// Revoke the role
		assert_ok!(TikiPallet::revoke_tiki(RuntimeOrigin::root(), user, TikiEnum::Dadger));

		// The score has decreased
		let final_score = TikiPallet::get_tiki_score(&user);
		assert!(final_score < initial_score);

		// It is not in the role list
		let tikis = TikiPallet::user_tikis(user);
		assert!(!tikis.contains(&TikiEnum::Dadger));
	});
}

#[test]
fn revoke_tiki_root_authority() {
	new_test_ext().execute_with(|| {
		let user = 14;

		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user, TikiEnum::Dadger));

		// Non-root cannot revoke
		assert_noop!(
			TikiPallet::revoke_tiki(RuntimeOrigin::signed(999), user, TikiEnum::Dadger),
			pezsp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn revoke_tiki_nonexistent_role() {
	new_test_ext().execute_with(|| {
		let user = 15;

		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));

		// The user does not have this role
		assert_noop!(
			TikiPallet::revoke_tiki(RuntimeOrigin::root(), user, TikiEnum::PisporêEwlehiyaSîber),
			Error::<Test>::RoleNotAssigned
		);
	});
}

// ============================================================================
// get_tiki_score Edge Cases (3 tests)
// ============================================================================

#[test]
fn get_tiki_score_zero_for_non_citizen() {
	new_test_ext().execute_with(|| {
		let user = 999;

		let score = TikiPallet::get_tiki_score(&user);
		assert_eq!(score, 0);
	});
}

#[test]
fn get_tiki_score_role_accumulation() {
	new_test_ext().execute_with(|| {
		let user = 16;

		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));

		// Initial: Welati = 10
		let score1 = TikiPallet::get_tiki_score(&user);
		assert_eq!(score1, 10);

		// Add Dadger (+150)
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user, TikiEnum::Dadger));
		let score2 = TikiPallet::get_tiki_score(&user);
		assert_eq!(score2, 160); // 10 + 150

		// Add Wezir (+100)
		assert_ok!(TikiPallet::grant_tiki(
			RuntimeOrigin::root(),
			user,
			TikiEnum::PisporêEwlehiyaSîber
		));
		let score3 = TikiPallet::get_tiki_score(&user);
		assert_eq!(score3, 260); // 10 + 150 + 100
	});
}

#[test]
fn get_tiki_score_revoke_decreases() {
	new_test_ext().execute_with(|| {
		let user = 17;

		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user, TikiEnum::Dadger));
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user, TikiEnum::Dozger));

		let score_before = TikiPallet::get_tiki_score(&user);
		assert_eq!(score_before, 280); // 10 + 150 + 120

		// Revoke one role
		assert_ok!(TikiPallet::revoke_tiki(RuntimeOrigin::root(), user, TikiEnum::Dadger));

		let score_after = TikiPallet::get_tiki_score(&user);
		assert_eq!(score_after, 130); // 10 + 120
	});
}

// ============================================================================
// Storage Consistency Tests (3 tests)
// ============================================================================

#[test]
fn user_tikis_updated_after_grant() {
	new_test_ext().execute_with(|| {
		let user = 18;

		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));

		let tikis_before = TikiPallet::user_tikis(user);
		assert_eq!(tikis_before.len(), 1); // Only Welati

		// Add a role
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user, TikiEnum::Dadger));

		// UserTikis was updated
		let tikis_after = TikiPallet::user_tikis(user);
		assert_eq!(tikis_after.len(), 2);
		assert!(tikis_after.contains(&TikiEnum::Dadger));
	});
}

#[test]
fn user_tikis_consistent_with_score() {
	new_test_ext().execute_with(|| {
		let user = 19;

		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user));
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user, TikiEnum::Dadger));
		assert_ok!(TikiPallet::grant_tiki(
			RuntimeOrigin::root(),
			user,
			TikiEnum::PisporêEwlehiyaSîber
		));

		// The UserTikis count should be consistent with the score
		let user_tikis = TikiPallet::user_tikis(user);
		let score = TikiPallet::get_tiki_score(&user);

		assert_eq!(user_tikis.len(), 3); // Welati + Dadger + Wezir
		assert_eq!(score, 260); // 10 + 150 + 100
	});
}

#[test]
fn multiple_users_independent_roles() {
	new_test_ext().execute_with(|| {
		let user1 = 20;
		let user2 = 21;

		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user1));
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), user2));

		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), user1, TikiEnum::Dadger));
		assert_ok!(TikiPallet::grant_tiki(
			RuntimeOrigin::root(),
			user2,
			TikiEnum::PisporêEwlehiyaSîber
		));

		// The roles are independent
		let tikis1 = TikiPallet::user_tikis(user1);
		let tikis2 = TikiPallet::user_tikis(user2);

		assert!(tikis1.contains(&TikiEnum::Dadger));
		assert!(!tikis1.contains(&TikiEnum::PisporêEwlehiyaSîber));

		assert!(tikis2.contains(&TikiEnum::PisporêEwlehiyaSîber));
		assert!(!tikis2.contains(&TikiEnum::Dadger));
	});
}

// =============================================================================
// WHO CAN TAKE AN OFFICE AWAY
// =============================================================================
//
// Granting was always bound to how a role is obtained. Taking away was not: every revocation
// went through `AdminOrigin`, which on the real chain is Root or the President or a council
// majority. So a council majority could strip the tiki from a President the country had
// elected. An asymmetry there is the whole difference between a check and a coup.

mod revocation {
	use super::*;

	/// The mock binds both `AdminOrigin` and `ImpeachmentOrigin` to root, so a test cannot
	/// tell them apart by origin. What it can tell apart is which roles each path accepts,
	/// which is the part that carries the rule.
	fn citizen(who: u64) {
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), who));
	}

	#[test]
	fn an_appointed_role_is_revoked_by_the_admin_path() {
		new_test_ext().execute_with(|| {
			citizen(2);
			assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), 2, TikiEnum::Dadger));

			assert_ok!(TikiPallet::revoke_tiki(RuntimeOrigin::root(), 2, TikiEnum::Dadger));
			assert!(!TikiPallet::has_tiki(&2, &TikiEnum::Dadger));
		});
	}

	#[test]
	fn an_elected_office_takes_the_impeachment_path() {
		new_test_ext().execute_with(|| {
			citizen(2);
			assert_ok!(TikiPallet::grant_elected_role(RuntimeOrigin::root(), 2, TikiEnum::Serok));
			assert_eq!(
				TikiPallet::role_assignment_type_of(2, TikiEnum::Serok),
				Some(RoleAssignmentType::Elected)
			);

			assert_ok!(TikiPallet::revoke_tiki(RuntimeOrigin::root(), 2, TikiEnum::Serok));
			assert!(!TikiPallet::has_tiki(&2, &TikiEnum::Serok));
		});
	}

	#[test]
	fn citizenship_cannot_be_taken_by_this_pallet_at_all() {
		new_test_ext().execute_with(|| {
			citizen(2);
			assert_noop!(
				TikiPallet::revoke_tiki(RuntimeOrigin::root(), 2, TikiEnum::Welati),
				Error::<Test>::RoleNotRevocable
			);
			assert!(TikiPallet::has_tiki(&2, &TikiEnum::Welati));
		});
	}

	#[test]
	fn a_role_with_no_recorded_provenance_falls_back_to_its_taxonomy() {
		// Roles seated before provenance was recorded still have a category, and the
		// revocation path has to follow it rather than defaulting to the widest origin.
		new_test_ext().execute_with(|| {
			citizen(2);
			assert_ok!(TikiPallet::grant_elected_role(RuntimeOrigin::root(), 2, TikiEnum::Serok));
			crate::RoleAssignmentTypeOf::<Test>::remove(2, TikiEnum::Serok);

			assert_ok!(TikiPallet::revoke_tiki(RuntimeOrigin::root(), 2, TikiEnum::Serok));
		});
	}
}

// =============================================================================
// TERMS
// =============================================================================
//
// The term's value belongs to whoever grants the role -- the ballot knows how long a
// parliament sits. What this pallet contributes is that the term is enforced: an expired role
// reads as absent from the moment it expires, whether or not anyone remembered to remove it.

mod terms {
	use super::*;

	fn citizen(who: u64) {
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), who));
	}

	#[test]
	fn a_role_with_no_term_never_expires() {
		new_test_ext().execute_with(|| {
			citizen(2);
			assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), 2, TikiEnum::Dadger));

			System::set_block_number(1_000_000);
			assert!(TikiPallet::has_tiki(&2, &TikiEnum::Dadger));
		});
	}

	#[test]
	fn an_expired_role_reads_as_absent_without_anyone_removing_it() {
		new_test_ext().execute_with(|| {
			citizen(2);
			assert_ok!(TikiPallet::internal_grant_role_until(&2, TikiEnum::Xezinedar, 100));

			System::set_block_number(100);
			assert!(TikiPallet::has_tiki(&2, &TikiEnum::Xezinedar));

			System::set_block_number(101);
			assert!(!TikiPallet::has_tiki(&2, &TikiEnum::Xezinedar));
		});
	}

	#[test]
	fn an_expired_officeholder_is_nobody_as_far_as_lookups_go() {
		// This is the read every other pallet makes: the treasury asks who holds the finance
		// portfolio. Reading the raw map would hand it to someone whose term ended.
		new_test_ext().execute_with(|| {
			citizen(2);
			assert_ok!(TikiPallet::internal_grant_role_until(&2, TikiEnum::Xezinedar, 100));

			assert_eq!(TikiPallet::current_holder(&TikiEnum::Xezinedar), Some(2));

			System::set_block_number(200);
			assert_eq!(TikiPallet::current_holder(&TikiEnum::Xezinedar), None);
			// The raw map still names them, which is exactly why nothing should read it.
			assert_eq!(TikiPallet::tiki_holder(TikiEnum::Xezinedar), Some(2));
		});
	}

	#[test]
	fn an_office_whose_term_ran_out_can_be_filled_again() {
		// Otherwise the one thing a term is meant to make possible -- replacing someone whose
		// time is up -- would be the one thing it blocks.
		new_test_ext().execute_with(|| {
			citizen(2);
			citizen(3);
			assert_ok!(TikiPallet::internal_grant_role_until(&2, TikiEnum::Xezinedar, 100));

			// While the term runs, the office is taken.
			assert_noop!(
				TikiPallet::internal_grant_role(&3, TikiEnum::Xezinedar),
				Error::<Test>::RoleAlreadyTaken
			);

			System::set_block_number(200);
			assert_ok!(TikiPallet::internal_grant_role(&3, TikiEnum::Xezinedar));

			assert_eq!(TikiPallet::current_holder(&TikiEnum::Xezinedar), Some(3));
			assert!(!TikiPallet::has_tiki(&2, &TikiEnum::Xezinedar));
		});
	}

	#[test]
	fn an_expired_role_stops_counting_towards_standing() {
		new_test_ext().execute_with(|| {
			citizen(2);
			let with_citizenship_only = TikiPallet::get_tiki_score(&2);
			// A qualification, not an office. Offices earn the holder nothing at all now, so
			// one would prove nothing here -- what is under test is that a term ending stops
			// the count, and only something that counts can show that.
			assert_ok!(TikiPallet::internal_grant_role_until(&2, TikiEnum::Mamoste, 100));
			assert!(TikiPallet::get_tiki_score(&2) > with_citizenship_only);

			System::set_block_number(200);
			assert_eq!(TikiPallet::get_tiki_score(&2), with_citizenship_only);
		});
	}

	#[test]
	fn revoking_clears_the_term_with_the_role() {
		new_test_ext().execute_with(|| {
			citizen(2);
			assert_ok!(TikiPallet::internal_grant_role_until(&2, TikiEnum::Dadger, 100));
			assert!(TikiPallet::tiki_expiry(2, TikiEnum::Dadger).is_some());

			assert_ok!(TikiPallet::revoke_tiki(RuntimeOrigin::root(), 2, TikiEnum::Dadger));
			assert!(TikiPallet::tiki_expiry(2, TikiEnum::Dadger).is_none());
		});
	}
}

// =============================================================================
// EARNED ROLES
// =============================================================================

mod earned {
	use super::*;
	use crate::EarnedRoleGranter;

	#[test]
	fn a_pallet_holding_the_evidence_can_award_an_earned_role() {
		new_test_ext().execute_with(|| {
			assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), 2));

			assert_ok!(<TikiPallet as EarnedRoleGranter<u64, TikiEnum>>::grant_earned(
				&2,
				TikiEnum::Axa
			));

			assert!(TikiPallet::has_tiki(&2, &TikiEnum::Axa));
			assert_eq!(
				TikiPallet::role_assignment_type_of(2, TikiEnum::Axa),
				Some(RoleAssignmentType::Earned)
			);
		});
	}

	#[test]
	fn awarding_the_same_role_again_is_not_a_failure() {
		// The caller is reporting that a threshold was crossed, and the count that crossed it
		// keeps going up. Erroring would make every referral after the twenty-fifth fail.
		new_test_ext().execute_with(|| {
			assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), 2));
			assert_ok!(<TikiPallet as EarnedRoleGranter<u64, TikiEnum>>::grant_earned(
				&2,
				TikiEnum::Axa
			));
			assert_ok!(<TikiPallet as EarnedRoleGranter<u64, TikiEnum>>::grant_earned(
				&2,
				TikiEnum::Axa
			));

			assert_eq!(
				TikiPallet::user_tikis(2).iter().filter(|t| **t == TikiEnum::Axa).count(),
				1
			);
		});
	}

	#[test]
	fn it_cannot_be_used_to_hand_out_offices_that_are_not_earned() {
		// A pallet that counts referrals has no business seating a judge or a president.
		new_test_ext().execute_with(|| {
			assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), 2));

			for forbidden in [TikiEnum::Serok, TikiEnum::Dadger, TikiEnum::Xezinedar] {
				assert_noop!(
					<TikiPallet as EarnedRoleGranter<u64, TikiEnum>>::grant_earned(&2, forbidden),
					Error::<Test>::InvalidRoleAssignmentMethod
				);
			}
		});
	}
}

// =============================================================================
// THE INVARIANT CAN FAIL
// =============================================================================
//
// `try_state` compares the three records of who holds what. That only means something if it
// can reject a bad state -- a check that always passes reads as coverage and is worse than
// none. Each test here breaks one thing and insists the invariant sees it.

#[cfg(feature = "try-runtime")]
mod invariant {
	use super::*;
	use crate::{CitizenNft, TikiHolder, UserTikis};
	use pezframe_support::traits::{Hooks, TryState, TryStateSelect};

	fn check() -> Result<(), pezsp_runtime::TryRuntimeError> {
		<TikiPallet as Hooks<u64>>::try_state(System::block_number())
	}

	fn assert_rejected(what: &str) {
		assert!(check().is_err(), "try_state accepted a state where {what}");
	}

	/// A citizen holding one single-holder office and one that may have several holders.
	///
	/// `Xezinedar` is in `is_unique_role`; `Dadger` is not, which is worth knowing when
	/// reading these -- a test that used a judgeship to check the reverse index would be
	/// checking nothing, because non-unique roles have no entry there by design.
	fn seated() {
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), 2));
		assert_ok!(TikiPallet::grant_tiki(RuntimeOrigin::root(), 2, TikiEnum::Xezinedar));
		assert_ok!(TikiPallet::grant_tiki(
			RuntimeOrigin::root(),
			2,
			TikiEnum::PisporêEwlehiyaSîber
		));
	}

	#[test]
	fn an_ordinary_state_passes() {
		new_test_ext().execute_with(|| {
			seated();
			assert_ok!(check());
			// And through the runtime's own entry point, the way try-runtime calls it.
			assert_ok!(AllPalletsWithSystem::try_state(
				System::block_number(),
				TryStateSelect::All
			));
		});
	}

	#[test]
	fn a_reverse_index_pointing_at_a_non_holder_is_caught() {
		new_test_ext().execute_with(|| {
			seated();
			// The office says account 3 holds it; account 3's own list does not.
			TikiHolder::<Test>::insert(TikiEnum::Serok, 3);
			assert_rejected("the reverse index named someone who does not hold the role");
		});
	}

	#[test]
	fn a_unique_office_missing_from_the_reverse_index_is_caught() {
		new_test_ext().execute_with(|| {
			seated();
			TikiHolder::<Test>::remove(TikiEnum::Xezinedar);
			assert_rejected("a single-holder office had no reverse index entry");
		});
	}

	#[test]
	fn a_unique_office_indexed_to_the_wrong_account_is_caught() {
		// The shape of a real failure: two accounts believing they hold the same office, with
		// every downstream check answering differently depending on which record it reads.
		new_test_ext().execute_with(|| {
			seated();
			TikiHolder::<Test>::insert(TikiEnum::Xezinedar, 3);
			assert_rejected("an office was indexed to an account other than its holder");
		});
	}

	#[test]
	fn roles_held_without_citizenship_are_caught() {
		new_test_ext().execute_with(|| {
			seated();
			CitizenNft::<Test>::remove(2);
			assert_rejected("an account held offices without being a citizen");
		});
	}

	#[test]
	fn a_duplicated_role_is_caught() {
		new_test_ext().execute_with(|| {
			seated();
			UserTikis::<Test>::mutate(2, |tikis| {
				let _ = tikis.try_push(TikiEnum::PisporêEwlehiyaSîber);
			});
			assert_rejected("an account held the same role twice");
		});
	}

	#[test]
	fn provenance_for_a_role_nobody_holds_is_caught() {
		new_test_ext().execute_with(|| {
			seated();
			crate::RoleAssignmentTypeOf::<Test>::insert(
				3,
				TikiEnum::Noter,
				RoleAssignmentType::Appointed,
			);
			assert_rejected("a grant was recorded for a role the account does not hold");
		});
	}

	#[test]
	fn provenance_that_contradicts_the_taxonomy_is_caught() {
		// Says an appointed office was won at a ballot. If that were allowed to stand, the
		// revocation path would read it and send an appointment to the court.
		new_test_ext().execute_with(|| {
			seated();
			crate::RoleAssignmentTypeOf::<Test>::insert(
				2,
				TikiEnum::Xezinedar,
				RoleAssignmentType::Elected,
			);
			assert_rejected("a role recorded a grant type it cannot be granted by");
		});
	}
}

#[test]
fn losing_citizenship_takes_the_offices_first() {
	// `identity-kyc::revoke_citizenship` logs a failure here rather than reverting, so if the
	// burn went first and failed, someone recorded as no longer a citizen would still be
	// holding the finance portfolio. Offices go first; a failure leaves an orphaned NFT
	// rather than an orphaned authority.
	new_test_ext().execute_with(|| {
		use pezpallet_identity_kyc::types::CitizenNftProvider;

		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), 2));
		// Seated the way an office is seated now: `grant_tiki` refuses the ones a nomination
		// confirms, and the Treasurer is one of them.
		assert_ok!(TikiPallet::internal_grant_role(&2, TikiEnum::Xezinedar));
		assert_eq!(TikiPallet::current_holder(&TikiEnum::Xezinedar), Some(2));

		assert_ok!(<TikiPallet as CitizenNftProvider<u64>>::burn_citizen_nft(&2));

		assert_eq!(TikiPallet::current_holder(&TikiEnum::Xezinedar), None);
		assert!(TikiPallet::user_tikis(2).is_empty());
		assert_eq!(TikiPallet::citizen_nft(2), None);
	});
}

#[test]
fn the_admin_call_cannot_seat_an_office_governance_owns() {
	new_test_ext().execute_with(|| {
		let who = 42u64;
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), who));

		// The cabinet is the Prime Minister's to fill, the Prime Minister is the President's,
		// and the bench answers to the court's own rules. Each writes into this register
		// through welati. This call answers to a wider origin than any of them, so it must
		// refuse -- otherwise the register can assert an office nobody constitutional decided.
		for office in [
			TikiEnum::WezireDarayiye,
			TikiEnum::WezireDad,
			TikiEnum::Wezir,
			TikiEnum::SerokWeziran,
			TikiEnum::EndameDiwane,
		] {
			assert_noop!(
				TikiPallet::grant_tiki(RuntimeOrigin::root(), who, office),
				Error::<Test>::SeatedByGovernance
			);
		}
	});
}

#[test]
fn the_admin_call_cannot_unseat_an_office_governance_owns() {
	new_test_ext().execute_with(|| {
		let minister = 43u64;
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), minister));

		// Seat it the way welati does, bypassing the extrinsic.
		assert_ok!(TikiPallet::internal_grant_role(&minister, TikiEnum::WezireDarayiye));

		// Taking it away has to go back through welati too. An origin that can revoke is an
		// origin that can replace: revoke then grant is the same as appointing.
		assert_noop!(
			TikiPallet::revoke_tiki(RuntimeOrigin::root(), minister, TikiEnum::WezireDarayiye),
			Error::<Test>::SeatedByGovernance
		);
		assert!(TikiPallet::has_tiki(&minister, &TikiEnum::WezireDarayiye));
	});
}

#[test]
fn the_admin_door_is_shut_on_offices_a_nomination_confirms() {
	new_test_ext().execute_with(|| {
		let who = 44u64;
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), who));

		// The Treasurer and the Ambassador used to be reachable here, on the grounds that this
		// call was their only door. It is not: `welati::tiki_for_role` maps both from an
		// `OfficialRole`, and `requires_parliament_approval` names them among the offices a
		// parliament must confirm. Shutting the shortcut leaves the proper road open.
		for office in [TikiEnum::Xezinedar, TikiEnum::Balyoz] {
			assert_noop!(
				TikiPallet::grant_tiki(RuntimeOrigin::root(), who, office),
				Error::<Test>::SeatedByGovernance
			);
			// And the road that remains still works.
			assert_ok!(TikiPallet::internal_grant_role(&who, office));
			assert!(TikiPallet::has_tiki(&who, &office));
		}
	});
}

#[test]
fn a_president_cannot_enlarge_his_own_share() {
	// The path this closes, measured before it was closed:
	//
	//   `AdminOrigin` on the People chain is `RootOrSerokOrCouncil`, so the sitting President
	//   satisfies it alone. `grant_tiki` refused only the offices `is_seated_by_governance`
	//   named, and the Treasurer and the Ambassador were not among them. Granting himself
	//   those two was worth a hundred and eighty points of standing, and every reward is
	//   divided by standing -- so the office that grants offices could raise its holder's
	//   share of the citizens' pot.
	//
	// Three things had to be true for that to work, and each is now false on its own: an
	// office contributes nothing to standing, nobody may grant to themselves, and those two
	// offices are seated by nomination rather than by this call.
	new_test_ext().execute_with(|| {
		let president = crate::mock::MOCK_SEROK;
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), president));
		assert_ok!(TikiPallet::internal_grant_role(&president, TikiEnum::Serok));

		let standing = TikiPallet::get_tiki_score(&president);

		// The presidency itself adds nothing: the table prices it at two hundred and the
		// holder's standing is citizenship alone.
		assert!(TikiPallet::has_tiki(&president, &TikiEnum::Serok));
		assert_eq!(TikiPallet::get_bonus_for_tiki(&TikiEnum::Serok), 200);
		assert_eq!(standing, TikiPallet::get_bonus_for_tiki(&TikiEnum::Welati));

		// Those two offices are no longer reachable by this call at all, for anyone.
		let someone = 8u64;
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), someone));
		for office in [TikiEnum::Xezinedar, TikiEnum::Balyoz] {
			assert_noop!(
				TikiPallet::grant_tiki(RuntimeOrigin::signed(president), someone, office),
				Error::<Test>::SeatedByGovernance
			);
		}

		// Nor anything else, office or qualification: an appointment has two parties.
		assert_noop!(
			TikiPallet::grant_tiki(RuntimeOrigin::signed(president), president, TikiEnum::Dadger),
			Error::<Test>::CannotGrantToSelf
		);

		// Someone else may still be given a qualification, and it still counts for them.
		let other = 9u64;
		assert_ok!(TikiPallet::grant_honorary_citizenship(RuntimeOrigin::root(), other));
		assert_ok!(TikiPallet::grant_tiki(
			RuntimeOrigin::signed(president),
			other,
			TikiEnum::Dadger
		));
		assert!(TikiPallet::get_tiki_score(&other) > TikiPallet::get_tiki_score(&president));

		// And his standing has not moved through any of it.
		assert_eq!(TikiPallet::get_tiki_score(&president), standing);
	});
}
