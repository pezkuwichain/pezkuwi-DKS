// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{
	mock::{new_test_ext, Perwerde as PerwerdePallet, RuntimeOrigin, System, Test},
	Event,
};
use pezframe_support::{assert_noop, assert_ok, pezpallet_prelude::Get, BoundedVec};
use pezsp_runtime::DispatchError;

fn create_bounded_vec<L: Get<u32>>(s: &[u8]) -> BoundedVec<u8, L> {
	s.to_vec().try_into().unwrap()
}

#[test]
fn create_course_works() {
	new_test_ext().execute_with(|| {
		// Admin olarak mock.rs'te TestAdminProvider içinde tanımladığımız hesabı kullanıyoruz.
		let admin_account_id = 0;

		// Eylem: Yetkili admin ile kurs oluştur.
		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin_account_id),
			create_bounded_vec(b"Blockchain 101"),
			create_bounded_vec(b"Giris seviyesi"),
			create_bounded_vec(b"http://example.com")
		));

		// Doğrulama
		assert!(crate::Courses::<Test>::contains_key(0));
		let course = crate::Courses::<Test>::get(0).unwrap();
		assert_eq!(course.owner, admin_account_id);
		System::assert_last_event(
			Event::CourseCreated { course_id: 0, owner: admin_account_id }.into(),
		);
	});
}

#[test]
fn create_course_fails_for_non_admin() {
	new_test_ext().execute_with(|| {
		// Admin (0) dışındaki bir hesap (2) kurs oluşturamaz.
		let non_admin = 2;
		assert_noop!(
			PerwerdePallet::create_course(
				RuntimeOrigin::signed(non_admin),
				create_bounded_vec(b"Hacking 101"),
				create_bounded_vec(b"Yetkisiz kurs"),
				create_bounded_vec(b"http://example.com")
			),
			DispatchError::BadOrigin
		);
	});
}

// ============================================================================
// ENROLL TESTS (8 tests)
// ============================================================================

#[test]
fn enroll_works() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		// Create course first
		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Rust Basics"),
			create_bounded_vec(b"Learn Rust"),
			create_bounded_vec(b"http://example.com")
		));

		// Student enrolls
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0));

		// Verify enrollment
		let enrollment = crate::Enrollments::<Test>::get((student, 0)).unwrap();
		assert_eq!(enrollment.student, student);
		assert_eq!(enrollment.course_id, 0);
		assert_eq!(enrollment.completed_at, None);
		assert_eq!(enrollment.points_earned, 0);

		// Verify StudentCourses updated
		let student_courses = crate::StudentCourses::<Test>::get(student);
		assert!(student_courses.contains(&0));

		System::assert_last_event(Event::StudentEnrolled { student, course_id: 0 }.into());
	});
}

#[test]
fn enroll_fails_for_nonexistent_course() {
	new_test_ext().execute_with(|| {
		let student = 1;
		assert_noop!(
			PerwerdePallet::enroll(RuntimeOrigin::signed(student), 999),
			crate::Error::<Test>::CourseNotFound
		);
	});
}

#[test]
fn enroll_fails_for_archived_course() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		// Create and archive course
		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Old Course"),
			create_bounded_vec(b"Archived"),
			create_bounded_vec(b"http://example.com")
		));
		assert_ok!(PerwerdePallet::archive_course(RuntimeOrigin::signed(admin), 0));

		// Try to enroll in archived course
		assert_noop!(
			PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0),
			crate::Error::<Test>::CourseNotActive
		);
	});
}

#[test]
fn enroll_fails_if_already_enrolled() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		// Create course
		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Description"),
			create_bounded_vec(b"http://example.com")
		));

		// First enrollment succeeds
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0));

		// Second enrollment fails
		assert_noop!(
			PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0),
			crate::Error::<Test>::AlreadyEnrolled
		);
	});
}

#[test]
fn multiple_students_can_enroll_same_course() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student1 = 1;
		let student2 = 2;
		let student3 = 3;

		// Create course
		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Popular Course"),
			create_bounded_vec(b"Many students"),
			create_bounded_vec(b"http://example.com")
		));

		// Multiple students enroll
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student1), 0));
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student2), 0));
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student3), 0));

		// Verify all enrollments
		assert!(crate::Enrollments::<Test>::contains_key((student1, 0)));
		assert!(crate::Enrollments::<Test>::contains_key((student2, 0)));
		assert!(crate::Enrollments::<Test>::contains_key((student3, 0)));
	});
}

#[test]
fn student_can_enroll_multiple_courses() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		// Create 3 courses
		for i in 0..3 {
			assert_ok!(PerwerdePallet::create_course(
				RuntimeOrigin::signed(admin),
				create_bounded_vec(format!("Course {i}").as_bytes()),
				create_bounded_vec(b"Description"),
				create_bounded_vec(b"http://example.com")
			));
		}

		// Student enrolls in all 3
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0));
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 1));
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 2));

		// Verify StudentCourses
		let student_courses = crate::StudentCourses::<Test>::get(student);
		assert_eq!(student_courses.len(), 3);
		assert!(student_courses.contains(&0));
		assert!(student_courses.contains(&1));
		assert!(student_courses.contains(&2));
	});
}

#[test]
fn enroll_fails_when_too_many_courses() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		// MaxCoursesPerStudent is 50, so create and enroll in 50 courses
		for i in 0..50 {
			assert_ok!(PerwerdePallet::create_course(
				RuntimeOrigin::signed(admin),
				create_bounded_vec(format!("Course {i}").as_bytes()),
				create_bounded_vec(b"Desc"),
				create_bounded_vec(b"http://example.com")
			));
			assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), i));
		}

		// Create one more course
		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course 50"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));

		// Enrollment should fail - student already enrolled in max courses
		assert_noop!(
			PerwerdePallet::enroll(RuntimeOrigin::signed(student), 50),
			crate::Error::<Test>::TooManyCourses
		);
	});
}

#[test]
fn enroll_event_emitted_correctly() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 5;

		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Test"),
			create_bounded_vec(b"Test"),
			create_bounded_vec(b"http://test.com")
		));

		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0));

		System::assert_last_event(Event::StudentEnrolled { student: 5, course_id: 0 }.into());
	});
}

// ============================================================================
// COMPLETE_COURSE TESTS (8 tests)
// ============================================================================

#[test]
fn complete_course_works() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;
		let points = 95;

		// Setup: Create course and enroll
		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0));

		// Complete the course (course owner completes for student)
		assert_ok!(PerwerdePallet::complete_course(
			RuntimeOrigin::signed(admin),
			student,
			0,
			points
		));

		// Verify completion
		let enrollment = crate::Enrollments::<Test>::get((student, 0)).unwrap();
		assert!(enrollment.completed_at.is_some());
		assert_eq!(enrollment.points_earned, points);

		System::assert_last_event(Event::CourseCompleted { student, course_id: 0, points }.into());
	});
}

#[test]
fn complete_course_fails_without_enrollment() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		// Create course but don't enroll
		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));

		// Try to complete without enrollment (admin tries to complete for non-enrolled student)
		assert_noop!(
			PerwerdePallet::complete_course(RuntimeOrigin::signed(admin), student, 0, 100),
			crate::Error::<Test>::NotEnrolled
		);
	});
}

#[test]
fn complete_course_fails_if_already_completed() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		// Setup
		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0));

		// First completion succeeds (admin completes)
		assert_ok!(PerwerdePallet::complete_course(RuntimeOrigin::signed(admin), student, 0, 85));

		// Second completion fails
		assert_noop!(
			PerwerdePallet::complete_course(RuntimeOrigin::signed(admin), student, 0, 90),
			crate::Error::<Test>::CourseAlreadyCompleted
		);
	});
}

#[test]
fn complete_course_with_zero_points() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0));

		// Complete with 0 points (failed course)
		assert_ok!(PerwerdePallet::complete_course(RuntimeOrigin::signed(admin), student, 0, 0));

		let enrollment = crate::Enrollments::<Test>::get((student, 0)).unwrap();
		assert_eq!(enrollment.points_earned, 0);
	});
}

#[test]
fn complete_course_with_max_allowed_points() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0));

		// Complete with maximum allowed points (MaxPointsPerCourse = 1000)
		assert_ok!(PerwerdePallet::complete_course(RuntimeOrigin::signed(admin), student, 0, 1000));

		let enrollment = crate::Enrollments::<Test>::get((student, 0)).unwrap();
		assert_eq!(enrollment.points_earned, 1000);
	});
}

#[test]
fn complete_course_fails_points_exceed_max() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0));

		// Points exceeding MaxPointsPerCourse (1000) should fail
		assert_noop!(
			PerwerdePallet::complete_course(RuntimeOrigin::signed(admin), student, 0, 1001),
			crate::Error::<Test>::PointsExceedMax
		);
	});
}

#[test]
fn multiple_students_complete_same_course() {
	new_test_ext().execute_with(|| {
		let admin = 0;

		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));

		// 3 students enroll and admin completes with different scores
		for i in 1u64..=3 {
			assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(i), 0));
			assert_ok!(PerwerdePallet::complete_course(
				RuntimeOrigin::signed(admin),
				i,
				0,
				(70 + (i * 10)) as u32
			));
		}

		// Verify each completion
		for i in 1u64..=3 {
			let enrollment = crate::Enrollments::<Test>::get((i, 0)).unwrap();
			assert!(enrollment.completed_at.is_some());
			assert_eq!(enrollment.points_earned, (70 + (i * 10)) as u32);
		}
	});
}

#[test]
fn student_completes_multiple_courses() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		// Create 3 courses
		for i in 0..3 {
			assert_ok!(PerwerdePallet::create_course(
				RuntimeOrigin::signed(admin),
				create_bounded_vec(format!("Course {i}").as_bytes()),
				create_bounded_vec(b"Desc"),
				create_bounded_vec(b"http://example.com")
			));
			assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), i));
		}

		// Complete all 3 (admin completes for student)
		assert_ok!(PerwerdePallet::complete_course(RuntimeOrigin::signed(admin), student, 0, 80));
		assert_ok!(PerwerdePallet::complete_course(RuntimeOrigin::signed(admin), student, 1, 90));
		assert_ok!(PerwerdePallet::complete_course(RuntimeOrigin::signed(admin), student, 2, 95));

		// Verify all completions
		for i in 0..3 {
			let enrollment = crate::Enrollments::<Test>::get((student, i)).unwrap();
			assert!(enrollment.completed_at.is_some());
		}
	});
}

#[test]
fn complete_course_event_emitted() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 7;
		let points = 88;

		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Test"),
			create_bounded_vec(b"Test"),
			create_bounded_vec(b"http://test.com")
		));
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0));
		assert_ok!(PerwerdePallet::complete_course(
			RuntimeOrigin::signed(admin),
			student,
			0,
			points
		));

		System::assert_last_event(
			Event::CourseCompleted { student: 7, course_id: 0, points: 88 }.into(),
		);
	});
}

// ============================================================================
// ARCHIVE_COURSE TESTS (4 tests)
// ============================================================================

#[test]
fn archive_course_works() {
	new_test_ext().execute_with(|| {
		let admin = 0;

		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));

		assert_ok!(PerwerdePallet::archive_course(RuntimeOrigin::signed(admin), 0));

		let course = crate::Courses::<Test>::get(0).unwrap();
		assert_eq!(course.status, crate::CourseStatus::Archived);

		System::assert_last_event(Event::CourseArchived { course_id: 0 }.into());
	});
}

#[test]
fn archive_course_fails_for_non_owner() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let other_user = 1;

		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));

		// Non-owner cannot archive
		assert_noop!(
			PerwerdePallet::archive_course(RuntimeOrigin::signed(other_user), 0),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn archive_course_fails_for_nonexistent_course() {
	new_test_ext().execute_with(|| {
		let admin = 0;

		assert_noop!(
			PerwerdePallet::archive_course(RuntimeOrigin::signed(admin), 999),
			crate::Error::<Test>::CourseNotFound
		);
	});
}

#[test]
fn archived_course_cannot_accept_new_enrollments() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		// Create and archive
		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));
		assert_ok!(PerwerdePallet::archive_course(RuntimeOrigin::signed(admin), 0));

		// Try to enroll - should fail
		assert_noop!(
			PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0),
			crate::Error::<Test>::CourseNotActive
		);
	});
}

// ============================================================================
// INTEGRATION & STORAGE TESTS (2 tests)
// ============================================================================

#[test]
fn storage_consistency_check() {
	new_test_ext().execute_with(|| {
		let admin = 0;
		let student = 1;

		// Create course
		assert_ok!(PerwerdePallet::create_course(
			RuntimeOrigin::signed(admin),
			create_bounded_vec(b"Course"),
			create_bounded_vec(b"Desc"),
			create_bounded_vec(b"http://example.com")
		));

		// Enroll
		assert_ok!(PerwerdePallet::enroll(RuntimeOrigin::signed(student), 0));

		// Check storage consistency
		assert!(crate::Courses::<Test>::contains_key(0));
		assert!(crate::Enrollments::<Test>::contains_key((student, 0)));

		let student_courses = crate::StudentCourses::<Test>::get(student);
		assert_eq!(student_courses.len(), 1);
		assert!(student_courses.contains(&0));

		let enrollment = crate::Enrollments::<Test>::get((student, 0)).unwrap();
		assert_eq!(enrollment.course_id, 0);
		assert_eq!(enrollment.student, student);
	});
}

#[test]
fn next_course_id_increments_correctly() {
	new_test_ext().execute_with(|| {
		let admin = 0;

		assert_eq!(crate::NextCourseId::<Test>::get(), 0);

		// Create 5 courses
		for i in 0..5 {
			assert_ok!(PerwerdePallet::create_course(
				RuntimeOrigin::signed(admin),
				create_bounded_vec(format!("Course {i}").as_bytes()),
				create_bounded_vec(b"Desc"),
				create_bounded_vec(b"http://example.com")
			));

			assert_eq!(crate::NextCourseId::<Test>::get(), i + 1);
		}

		// Verify all courses exist
		for i in 0..5 {
			assert!(crate::Courses::<Test>::contains_key(i));
		}
	});
}
