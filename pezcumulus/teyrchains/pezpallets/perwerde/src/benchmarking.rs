// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-perwerde

use super::{Pezpallet as Perwerde, *};
use pezframe_benchmarking::v2::*;
use pezframe_support::{pezpallet_prelude::Get, BoundedVec};
use pezframe_system::RawOrigin;
extern crate alloc;

// Helper function to create BoundedVec in benchmarks
fn create_bounded_vec<L: Get<u32>>(s: &[u8]) -> BoundedVec<u8, L> {
	s.to_vec().try_into().unwrap()
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create_course() {
		let name: BoundedVec<u8, T::MaxCourseNameLength> =
			create_bounded_vec(b"Bizinikiwi training");
		let description: BoundedVec<u8, T::MaxCourseDescLength> =
			create_bounded_vec(b"This training covers Bizinikiwi basics.");
		let content_link: BoundedVec<u8, T::MaxCourseLinkLength> =
			create_bounded_vec(b"http://example.com");

		// In benchmark environment, AdminOrigin is bypassed
		// We use Root origin which will satisfy the origin check

		#[extrinsic_call]
		create_course(RawOrigin::Root, name.clone(), description.clone(), content_link.clone());

		assert!(Courses::<T>::get(0).is_some());
	}

	#[benchmark]
	fn enroll() {
		let student: T::AccountId = whitelisted_caller();
		let course_id = 0;

		// Setup: Create a course first using root
		Perwerde::<T>::create_course(
			RawOrigin::Root.into(),
			create_bounded_vec(b"Benchmark Course"),
			create_bounded_vec(b"Description"),
			create_bounded_vec(b"Link"),
		)
		.unwrap();

		#[extrinsic_call]
		enroll(RawOrigin::Signed(student.clone()), course_id);

		assert!(Enrollments::<T>::get((student, course_id)).is_some());
	}

	#[benchmark]
	fn complete_course() {
		let student: T::AccountId = whitelisted_caller();
		let course_id = 0;
		let points = 10;

		// Setup: Create course and enroll student
		// Root creates the course via AdminOrigin
		Perwerde::<T>::create_course(
			RawOrigin::Root.into(),
			create_bounded_vec(b"Benchmark Course"),
			create_bounded_vec(b"Description"),
			create_bounded_vec(b"Link"),
		)
		.unwrap();
		Perwerde::<T>::enroll(RawOrigin::Signed(student.clone()).into(), course_id).unwrap();

		// Get the actual owner from the created course
		let course = Courses::<T>::get(course_id).unwrap();
		let owner = course.owner;

		// complete_course requires the owner to sign, not root
		#[extrinsic_call]
		complete_course(RawOrigin::Signed(owner), student.clone(), course_id, points);

		let enrollment = Enrollments::<T>::get((student, course_id)).unwrap();
		assert!(enrollment.completed_at.is_some());
		assert_eq!(enrollment.points_earned, points);
	}

	#[benchmark]
	fn archive_course() {
		let course_id = 0;

		// Setup: Create course first
		Perwerde::<T>::create_course(
			RawOrigin::Root.into(),
			create_bounded_vec(b"Benchmark Course"),
			create_bounded_vec(b"Description"),
			create_bounded_vec(b"Link"),
		)
		.unwrap();

		// archive_course requires AdminOrigin (which is Root in our config)
		// The AdminOrigin::try_origin for Root returns the admin account (Alice)
		// which matches the course owner from create_course
		#[extrinsic_call]
		archive_course(RawOrigin::Root, course_id);

		let course = Courses::<T>::get(course_id).unwrap();
		assert_eq!(course.status, CourseStatus::Archived);
	}

	impl_benchmark_test_suite!(Perwerde, crate::mock::new_test_ext(), crate::mock::Test);
}
