// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Tests for pezpallet-perwerde.
//!
//! What this pallet issues is not a mark, it is standing: education points enter the trust
//! score at a weight of three hundred, and trust decides who may stand for office. So most of
//! what follows is about who can create that standing and how hard it is to create it out of
//! nothing.

use crate::{
	mock::{new_test_ext, Perwerde as PerwerdePallet, RuntimeOrigin, System, Test},
	CourseStatus, Error, Event,
};
use pezframe_support::{assert_noop, assert_ok, pezpallet_prelude::Get, BoundedVec};
use pezsp_runtime::DispatchError;

/// The account `TestAdminProvider` accepts.
const ADMIN: u64 = 0;
const TEACHER: u64 = 1;
const STUDENT: u64 = 2;
const OTHER: u64 = 3;

fn bounded<L: Get<u32>>(s: &[u8]) -> BoundedVec<u8, L> {
	s.to_vec().try_into().unwrap()
}

/// Open a course worth `points`, taught by `TEACHER`.
fn open_course(points: u32) -> u32 {
	let id = PerwerdePallet::next_course_id();
	assert_ok!(PerwerdePallet::create_course(
		RuntimeOrigin::signed(ADMIN),
		TEACHER,
		bounded(b"Blockchain 101"),
		bounded(b"An introduction"),
		bounded(b"http://example.com"),
		points,
	));
	id
}

/// Take a course all the way through: enrol, pass, submit, and have the board ratify.
fn run_course_for(student: u64, points: u32) -> u32 {
	let id = open_course(points);
	assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), id));
	assert_ok!(PerwerdePallet::record_result(RuntimeOrigin::signed(TEACHER), student, id, true));
	System::set_block_number(System::block_number() + crate::mock::MinCourseDuration::get());
	assert_ok!(PerwerdePallet::submit_results(RuntimeOrigin::signed(TEACHER), id));
	ratify_fully(id);
	id
}

/// Sign off a course with a full board, none of them the teacher.
fn ratify_fully(course_id: u32) {
	for ratifier in 100..(100 + crate::mock::RatificationsRequired::get() as u64) {
		assert_ok!(PerwerdePallet::ratify_results(RuntimeOrigin::signed(ratifier), course_id));
	}
}

// =============================================================================
// NOBODY GRADES THEMSELVES
// =============================================================================

#[test]
fn a_teacher_cannot_enrol_in_their_own_course() {
	// Without this the whole pallet is a printing press: open a course, enrol yourself, award
	// yourself what it is worth, and repeat until the trust score is whatever you wanted.
	new_test_ext().execute_with(|| {
		let id = open_course(1000);
		assert_noop!(
			PerwerdePallet::enroll(RuntimeOrigin::signed(TEACHER), id),
			Error::<Test>::OwnerCannotEnrol
		);
	});
}

#[test]
fn a_teacher_cannot_ratify_their_own_course() {
	new_test_ext().execute_with(|| {
		let id = open_course(1000);
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(STUDENT), id));
		System::set_block_number(System::block_number() + crate::mock::MinCourseDuration::get());
		assert_ok!(PerwerdePallet::submit_results(RuntimeOrigin::signed(TEACHER), id));

		assert_noop!(
			PerwerdePallet::ratify_results(RuntimeOrigin::signed(TEACHER), id),
			Error::<Test>::OwnerCannotRatify
		);
	});
}

#[test]
fn what_a_course_is_worth_is_fixed_when_it_opens() {
	// The teacher used to choose the number for each student, up to the maximum. That let one
	// person give a friend ten times what somebody else got for the same work, and the
	// difference was political standing.
	new_test_ext().execute_with(|| {
		let id = run_course_for(STUDENT, 400);

		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), 400);
		assert_eq!(PerwerdePallet::courses(id).unwrap().points, 400);
	});
}

#[test]
fn a_course_cannot_be_worth_more_than_the_maximum() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PerwerdePallet::create_course(
				RuntimeOrigin::signed(ADMIN),
				TEACHER,
				bounded(b"Too rich"),
				bounded(b"..."),
				bounded(b"..."),
				crate::mock::MaxPointsPerCourse::get() + 1,
			),
			Error::<Test>::PointsExceedMax
		);
	});
}

#[test]
fn opening_a_course_needs_the_admin_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PerwerdePallet::create_course(
				RuntimeOrigin::signed(OTHER),
				TEACHER,
				bounded(b"Unofficial"),
				bounded(b"..."),
				bounded(b"..."),
				100,
			),
			DispatchError::BadOrigin
		);
	});
}

// =============================================================================
// RESULTS ARE DRAFTS UNTIL A BOARD SAYS OTHERWISE
// =============================================================================

#[test]
fn a_recorded_result_awards_nothing_on_its_own() {
	new_test_ext().execute_with(|| {
		let id = open_course(500);
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(STUDENT), id));
		assert_ok!(PerwerdePallet::record_result(
			RuntimeOrigin::signed(TEACHER),
			STUDENT,
			id,
			true
		));

		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), 0);
		assert_eq!(PerwerdePallet::completed_courses(STUDENT), 0);
	});
}

#[test]
fn a_draft_can_be_changed_while_the_course_runs() {
	new_test_ext().execute_with(|| {
		let id = open_course(500);
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(STUDENT), id));
		assert_ok!(PerwerdePallet::record_result(
			RuntimeOrigin::signed(TEACHER),
			STUDENT,
			id,
			true
		));
		assert_ok!(PerwerdePallet::record_result(
			RuntimeOrigin::signed(TEACHER),
			STUDENT,
			id,
			false
		));

		System::set_block_number(System::block_number() + crate::mock::MinCourseDuration::get());
		assert_ok!(PerwerdePallet::submit_results(RuntimeOrigin::signed(TEACHER), id));
		ratify_fully(id);

		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), 0);
	});
}

#[test]
fn the_points_become_real_when_the_board_is_complete() {
	new_test_ext().execute_with(|| {
		let id = open_course(500);
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(STUDENT), id));
		assert_ok!(PerwerdePallet::record_result(
			RuntimeOrigin::signed(TEACHER),
			STUDENT,
			id,
			true
		));
		System::set_block_number(System::block_number() + crate::mock::MinCourseDuration::get());
		assert_ok!(PerwerdePallet::submit_results(RuntimeOrigin::signed(TEACHER), id));

		// One short of the board: still nothing.
		for ratifier in 100..104u64 {
			assert_ok!(PerwerdePallet::ratify_results(RuntimeOrigin::signed(ratifier), id));
		}
		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), 0);
		assert_eq!(PerwerdePallet::courses(id).unwrap().status, CourseStatus::AwaitingRatification);

		assert_ok!(PerwerdePallet::ratify_results(RuntimeOrigin::signed(104), id));
		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), 500);
		assert_eq!(PerwerdePallet::courses(id).unwrap().status, CourseStatus::Completed);
	});
}

#[test]
fn one_teacher_cannot_stand_in_for_a_board() {
	new_test_ext().execute_with(|| {
		let id = open_course(500);
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(STUDENT), id));
		System::set_block_number(System::block_number() + crate::mock::MinCourseDuration::get());
		assert_ok!(PerwerdePallet::submit_results(RuntimeOrigin::signed(TEACHER), id));

		assert_ok!(PerwerdePallet::ratify_results(RuntimeOrigin::signed(100), id));
		assert_noop!(
			PerwerdePallet::ratify_results(RuntimeOrigin::signed(100), id),
			Error::<Test>::AlreadyRatified
		);
	});
}

// =============================================================================
// A COURSE TAKES TIME
// =============================================================================

#[test]
fn results_cannot_be_submitted_before_the_course_has_run() {
	// A course that could be opened and closed in an afternoon is a way of printing standing.
	new_test_ext().execute_with(|| {
		let id = open_course(500);
		assert_noop!(
			PerwerdePallet::submit_results(RuntimeOrigin::signed(TEACHER), id),
			Error::<Test>::TooEarlyToSubmit
		);
	});
}

#[test]
fn a_course_left_unratified_expires_and_awards_nothing() {
	new_test_ext().execute_with(|| {
		let id = open_course(500);
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(STUDENT), id));
		assert_ok!(PerwerdePallet::record_result(
			RuntimeOrigin::signed(TEACHER),
			STUDENT,
			id,
			true
		));
		System::set_block_number(System::block_number() + crate::mock::MinCourseDuration::get());
		assert_ok!(PerwerdePallet::submit_results(RuntimeOrigin::signed(TEACHER), id));

		System::set_block_number(System::block_number() + crate::mock::MaxCourseDuration::get());
		// Permissionless: it records what the calendar already settled.
		assert_ok!(PerwerdePallet::expire_course(RuntimeOrigin::signed(OTHER), id));

		assert_eq!(PerwerdePallet::courses(id).unwrap().status, CourseStatus::Expired);
		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), 0);
	});
}

#[test]
fn a_course_still_within_its_year_cannot_be_expired() {
	new_test_ext().execute_with(|| {
		let id = open_course(500);
		assert_noop!(
			PerwerdePallet::expire_course(RuntimeOrigin::signed(OTHER), id),
			Error::<Test>::NotYetExpired
		);
	});
}

// =============================================================================
// STUDY IS FREE; WHAT IT IS WORTH IN STANDING IS NOT
// =============================================================================

#[test]
fn beyond_the_limit_a_student_keeps_learning_and_stops_scoring() {
	// The old bound stopped a citizen enrolling at all once they had taken enough courses --
	// a lifelong learner locked out of learning. The cap belongs on the political weight, not
	// on the studying.
	new_test_ext().execute_with(|| {
		let limit = crate::mock::RewardedCourseLimit::get();
		for _ in 0..limit {
			run_course_for(STUDENT, 100);
		}
		let capped = PerwerdePallet::perwerde_score(STUDENT);
		assert_eq!(capped, 100 * limit);

		run_course_for(STUDENT, 100);

		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), capped, "the cap did not hold");
		assert_eq!(PerwerdePallet::rewarded_courses(STUDENT), limit);
		// The record still says they completed it.
		assert_eq!(PerwerdePallet::completed_courses(STUDENT), limit + 1);
	});
}

// =============================================================================
// SEEDING THE EXAMINER CORPS
// =============================================================================

#[test]
fn the_minister_may_seed_teachers_up_to_the_cap() {
	// Ratifying needs teachers and being a teacher is earned by completing courses, so without
	// this the pallet starts locked: no course can be ratified, so nobody can earn the role
	// that ratifies.
	new_test_ext().execute_with(|| {
		for who in 200..(200 + crate::mock::MaxHonoraryMamoste::get() as u64) {
			assert_ok!(PerwerdePallet::appoint_honorary_mamoste(RuntimeOrigin::root(), who));
		}
		assert_eq!(
			PerwerdePallet::honorary_mamoste_count(),
			crate::mock::MaxHonoraryMamoste::get()
		);

		assert_noop!(
			PerwerdePallet::appoint_honorary_mamoste(RuntimeOrigin::root(), 999),
			Error::<Test>::HonoraryMamosteLimitReached
		);
	});
}

#[test]
fn how_a_teacher_got_the_role_is_readable() {
	new_test_ext().execute_with(|| {
		assert_ok!(PerwerdePallet::appoint_honorary_mamoste(RuntimeOrigin::root(), 200));

		assert!(PerwerdePallet::is_honorary_mamoste(200).is_some());
		assert!(PerwerdePallet::is_honorary_mamoste(TEACHER).is_none());
	});
}

#[test]
fn seeding_a_teacher_is_the_ministers_alone() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PerwerdePallet::appoint_honorary_mamoste(RuntimeOrigin::signed(OTHER), 200),
			DispatchError::BadOrigin
		);
	});
}

// =============================================================================
// FRAUD
// =============================================================================

#[test]
fn the_court_can_annul_a_course_and_take_back_what_it_gave() {
	// Everything else in the state can be undone by the court -- an office, a citizenship --
	// and a credential should be no different. Standing bought from a teacher is standing all
	// the same until somebody can remove it.
	new_test_ext().execute_with(|| {
		let id = run_course_for(STUDENT, 500);
		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), 500);

		assert_ok!(PerwerdePallet::report_course_fraud(RuntimeOrigin::root(), id));
		assert_ok!(PerwerdePallet::annul_course(RuntimeOrigin::root(), id));

		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), 0);
		assert_eq!(PerwerdePallet::rewarded_courses(STUDENT), 0);
		assert_eq!(PerwerdePallet::courses(id).unwrap().status, CourseStatus::Annulled);
	});
}

#[test]
fn the_board_that_signed_it_is_recorded() {
	// Ratifying is vouching. If the court finds the results were not real, that is on the
	// board as much as on whoever wrote them.
	new_test_ext().execute_with(|| {
		let id = run_course_for(STUDENT, 500);
		assert_ok!(PerwerdePallet::report_course_fraud(RuntimeOrigin::root(), id));
		assert_ok!(PerwerdePallet::annul_course(RuntimeOrigin::root(), id));

		for ratifier in 100..105u64 {
			assert_eq!(PerwerdePallet::annulled_ratifications(ratifier), 1);
		}
		assert_eq!(PerwerdePallet::annulled_ratifications(TEACHER), 0);
	});
}

#[test]
fn the_court_only_annuls_what_the_minister_brought() {
	new_test_ext().execute_with(|| {
		let id = run_course_for(STUDENT, 500);
		assert_noop!(
			PerwerdePallet::annul_course(RuntimeOrigin::root(), id),
			Error::<Test>::NotUnderReview
		);
	});
}

#[test]
fn annulment_is_the_courts_and_not_the_ministers() {
	new_test_ext().execute_with(|| {
		let id = run_course_for(STUDENT, 500);
		assert_ok!(PerwerdePallet::report_course_fraud(RuntimeOrigin::root(), id));
		assert_noop!(
			PerwerdePallet::annul_course(RuntimeOrigin::signed(OTHER), id),
			DispatchError::BadOrigin
		);
	});
}

// =============================================================================
// THE RECORD
// =============================================================================

#[test]
fn enrolling_twice_is_refused() {
	new_test_ext().execute_with(|| {
		let id = open_course(100);
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(STUDENT), id));
		assert_noop!(
			PerwerdePallet::enroll(RuntimeOrigin::signed(STUDENT), id),
			Error::<Test>::AlreadyEnrolled
		);
	});
}

#[test]
fn only_the_teacher_records_results() {
	new_test_ext().execute_with(|| {
		let id = open_course(100);
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(STUDENT), id));
		assert_noop!(
			PerwerdePallet::record_result(RuntimeOrigin::signed(OTHER), STUDENT, id, true),
			Error::<Test>::NotCourseOwner
		);
	});
}

#[test]
fn a_course_that_was_opened_is_announced_with_what_it_is_worth() {
	new_test_ext().execute_with(|| {
		let id = open_course(250);
		System::assert_has_event(
			Event::CourseCreated { course_id: id, owner: TEACHER, points: 250 }.into(),
		);
	});
}

// =============================================================================
// THE INVARIANT CAN FAIL
// =============================================================================

#[cfg(feature = "try-runtime")]
mod invariant {
	use super::*;
	use crate::{CourseRatifiers, PerwerdeScores, RewardedCourses};
	use pezframe_support::traits::Hooks;

	fn check() -> Result<(), pezsp_runtime::TryRuntimeError> {
		<PerwerdePallet as Hooks<u64>>::try_state(System::block_number())
	}

	fn assert_rejected(what: &str) {
		assert!(check().is_err(), "try_state accepted a state where {what}");
	}

	#[test]
	fn an_ordinary_state_passes() {
		new_test_ext().execute_with(|| {
			run_course_for(STUDENT, 300);
			assert_ok!(check());
		});
	}

	#[test]
	fn a_score_ahead_of_the_courses_behind_it_is_caught() {
		// Worth three hundred in the trust formula, and trust is candidacy for office. A score
		// that quietly ran ahead would be standing invented out of nothing, and it would look
		// exactly like standing that had been earned.
		new_test_ext().execute_with(|| {
			run_course_for(STUDENT, 300);
			PerwerdeScores::<Test>::mutate(STUDENT, |s| *s += 1);
			assert_rejected("a student's score exceeded what their courses awarded");
		});
	}

	#[test]
	fn a_miscounted_rewarded_course_is_caught() {
		new_test_ext().execute_with(|| {
			run_course_for(STUDENT, 300);
			RewardedCourses::<Test>::mutate(STUDENT, |n| *n += 1);
			assert_rejected("a student was credited with more rewarded courses than awards");
		});
	}

	#[test]
	fn a_course_closed_without_a_full_board_is_caught() {
		new_test_ext().execute_with(|| {
			let id = run_course_for(STUDENT, 300);
			crate::RatificationCount::<Test>::insert(id, 1);
			assert_rejected("a course was closed on fewer signatures than the board requires");
		});
	}

	#[test]
	fn a_teacher_among_their_own_ratifiers_is_caught() {
		new_test_ext().execute_with(|| {
			let id = run_course_for(STUDENT, 300);
			CourseRatifiers::<Test>::insert(id, TEACHER, ());
			assert_rejected("a teacher ratified their own course");
		});
	}

	#[test]
	fn seeding_past_the_cap_is_caught() {
		new_test_ext().execute_with(|| {
			crate::HonoraryMamosteCount::<Test>::put(crate::mock::MaxHonoraryMamoste::get() + 1);
			assert_rejected("more teachers were seeded than the minister may seed");
		});
	}
}

// =============================================================================
// A CLASS IS READ BY ITS OWN PREFIX
// =============================================================================

#[test]
fn closing_one_course_awards_only_its_own_class() {
	// `Enrollments` used to be one map under a `(student, course)` tuple, which cannot be
	// prefixed -- so closing a course walked every enrolment on the chain and filtered by
	// `course_id` in memory. The filter was correct, but the cost was the whole register and
	// the comment claimed otherwise. The key is now `(course, student)`, so a class is a
	// prefix read; this pins that the two courses stay separate under the new layout.
	new_test_ext().execute_with(|| {
		let first = open_course(40);
		let second = open_course(70);

		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(STUDENT), first));
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(OTHER), second));
		assert_ok!(PerwerdePallet::record_result(
			RuntimeOrigin::signed(TEACHER),
			STUDENT,
			first,
			true
		));
		assert_ok!(PerwerdePallet::record_result(
			RuntimeOrigin::signed(TEACHER),
			OTHER,
			second,
			true
		));

		System::set_block_number(System::block_number() + crate::mock::MinCourseDuration::get());
		assert_ok!(PerwerdePallet::submit_results(RuntimeOrigin::signed(TEACHER), first));
		ratify_fully(first);

		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), 40, "the class that closed");
		assert_eq!(
			PerwerdePallet::perwerde_score(OTHER),
			0,
			"a student on another course must not be awarded by it"
		);
		assert_eq!(crate::Enrollments::<Test>::get(second, OTHER).unwrap().points_awarded, 0);
	});
}

#[test]
fn annulling_one_course_leaves_the_other_standing() {
	// The reversal is a prefix read too. If it ever walked the whole register again, a student who
	// earned points elsewhere would lose them to somebody else's fraud.
	new_test_ext().execute_with(|| {
		let honest = run_course_for(STUDENT, 40);
		let fraudulent = run_course_for(OTHER, 70);

		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), 40);
		assert_eq!(PerwerdePallet::perwerde_score(OTHER), 70);

		assert_ok!(PerwerdePallet::report_course_fraud(RuntimeOrigin::root(), fraudulent));
		assert_ok!(PerwerdePallet::annul_course(RuntimeOrigin::root(), fraudulent));

		assert_eq!(PerwerdePallet::perwerde_score(OTHER), 0, "the annulled course is reversed");
		assert_eq!(PerwerdePallet::perwerde_score(STUDENT), 40, "and nobody else pays for it");
		assert_eq!(crate::Courses::<Test>::get(honest).unwrap().status, CourseStatus::Completed);
	});
}
