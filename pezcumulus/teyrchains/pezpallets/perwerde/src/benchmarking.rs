// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarks for `pezpallet-perwerde`.
//!
//! One benchmark per call. The two that are not constant -- closing a course and annulling
//! one -- are measured over the class, because both walk it: `ratify_results` awards every
//! student who passed on the final ratification, and `annul_course` takes those awards back.

#![cfg(feature = "runtime-benchmarks")]

use super::{Pezpallet as Perwerde, *};
use pezframe_benchmarking::v2::*;
use pezframe_support::{assert_ok, pezpallet_prelude::Get, BoundedVec};
use pezframe_system::RawOrigin;
use pezsp_runtime::traits::Saturating;

extern crate alloc;
use alloc::vec::Vec;

fn bounded<L: Get<u32>>(s: &[u8]) -> BoundedVec<u8, L> {
	s.to_vec().try_into().unwrap()
}

/// Open a course owned by `owner` and return its id.
fn open_course<T: Config>(owner: &T::AccountId) -> u32 {
	let course_id = NextCourseId::<T>::get();
	assert_ok!(Perwerde::<T>::create_course(
		RawOrigin::Root.into(),
		owner.clone(),
		bounded(b"Benchmark Course"),
		bounded(b"Description"),
		bounded(b"Link"),
		10,
	));
	course_id
}

/// Enrol `count` students on `course_id` and record them all as passing.
fn fill_class<T: Config>(course_id: u32, owner: &T::AccountId, count: u32) -> Vec<T::AccountId> {
	let mut students = Vec::new();
	for i in 0..count {
		let student: T::AccountId = account("student", i, 0);
		assert_ok!(Perwerde::<T>::enroll(RawOrigin::Signed(student.clone()).into(), course_id));
		assert_ok!(Perwerde::<T>::record_result(
			RawOrigin::Signed(owner.clone()).into(),
			student.clone(),
			course_id,
			true,
		));
		students.push(student);
	}
	students
}

/// Make `count` teachers through the honorary route, which is the only path that does not
/// itself require a course to have been completed.
fn make_teachers<T: Config>(count: u32) -> Vec<T::AccountId> {
	let mut teachers = Vec::new();
	for i in 0..count {
		let teacher: T::AccountId = account("mamoste", i, 0);
		assert_ok!(Perwerde::<T>::appoint_honorary_mamoste(
			RawOrigin::Root.into(),
			teacher.clone()
		));
		teachers.push(teacher);
	}
	teachers
}

/// Move far enough forward that a course may have its results submitted.
fn past_min_duration<T: Config>() {
	let now = pezframe_system::Pezpallet::<T>::block_number();
	pezframe_system::Pezpallet::<T>::set_block_number(
		now.saturating_add(T::MinCourseDuration::get()),
	);
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create_course() {
		let owner: T::AccountId = whitelisted_caller();

		#[extrinsic_call]
		_(
			RawOrigin::Root,
			owner.clone(),
			bounded::<T::MaxCourseNameLength>(b"Benchmark Course"),
			bounded::<T::MaxCourseDescLength>(b"Description"),
			bounded::<T::MaxCourseLinkLength>(b"Link"),
			10u32,
		);

		assert!(Courses::<T>::contains_key(0));
	}

	#[benchmark]
	fn enroll() {
		let owner: T::AccountId = account("owner", 0, 0);
		let student: T::AccountId = whitelisted_caller();
		let course_id = open_course::<T>(&owner);

		#[extrinsic_call]
		_(RawOrigin::Signed(student.clone()), course_id);

		assert!(Enrollments::<T>::contains_key(course_id, &student));
	}

	#[benchmark]
	fn record_result() {
		let owner: T::AccountId = whitelisted_caller();
		let student: T::AccountId = account("student", 0, 0);
		let course_id = open_course::<T>(&owner);
		assert_ok!(Perwerde::<T>::enroll(RawOrigin::Signed(student.clone()).into(), course_id));

		#[extrinsic_call]
		_(RawOrigin::Signed(owner), student.clone(), course_id, true);

		assert_eq!(Enrollments::<T>::get(course_id, &student).unwrap().passed, Some(true));
	}

	#[benchmark]
	fn submit_results() {
		let owner: T::AccountId = whitelisted_caller();
		let course_id = open_course::<T>(&owner);
		fill_class::<T>(course_id, &owner, 1);
		past_min_duration::<T>();

		#[extrinsic_call]
		_(RawOrigin::Signed(owner), course_id);

		assert_eq!(
			Courses::<T>::get(course_id).unwrap().status,
			CourseStatus::AwaitingRatification
		);
	}

	/// `c` is the class size: the last ratification closes the course and awards every one
	/// of them.
	#[benchmark]
	fn ratify_results(c: Linear<1, { T::MaxStudentsPerCourse::get() }>) {
		let owner: T::AccountId = account("owner", 0, 0);
		let course_id = open_course::<T>(&owner);
		fill_class::<T>(course_id, &owner, c);
		past_min_duration::<T>();
		assert_ok!(Perwerde::<T>::submit_results(
			RawOrigin::Signed(owner.clone()).into(),
			course_id
		));

		let required = T::RatificationsRequired::get();
		let teachers = make_teachers::<T>(required);
		// All but the last, so the measured call is the one that closes the course.
		for teacher in teachers.iter().take(required.saturating_sub(1) as usize) {
			assert_ok!(Perwerde::<T>::ratify_results(
				RawOrigin::Signed(teacher.clone()).into(),
				course_id
			));
		}
		let last = teachers.last().expect("at least one ratifier is required").clone();

		#[extrinsic_call]
		_(RawOrigin::Signed(last), course_id);

		assert_eq!(Courses::<T>::get(course_id).unwrap().status, CourseStatus::Completed);
	}

	#[benchmark]
	fn expire_course() {
		let owner: T::AccountId = account("owner", 0, 0);
		let caller: T::AccountId = whitelisted_caller();
		let course_id = open_course::<T>(&owner);
		let now = pezframe_system::Pezpallet::<T>::block_number();
		pezframe_system::Pezpallet::<T>::set_block_number(
			now.saturating_add(T::MaxCourseDuration::get()).saturating_add(1u32.into()),
		);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), course_id);

		assert_eq!(Courses::<T>::get(course_id).unwrap().status, CourseStatus::Expired);
	}

	#[benchmark]
	fn appoint_honorary_mamoste() {
		let teacher: T::AccountId = whitelisted_caller();

		#[extrinsic_call]
		_(RawOrigin::Root, teacher.clone());

		assert!(HonoraryMamoste::<T>::contains_key(&teacher));
	}

	#[benchmark]
	fn report_course_fraud() {
		let owner: T::AccountId = account("owner", 0, 0);
		let course_id = open_course::<T>(&owner);

		#[extrinsic_call]
		_(RawOrigin::Root, course_id);

		assert!(CoursesUnderReview::<T>::contains_key(course_id));
	}

	/// `c` is the class size: every award the course made is taken back.
	#[benchmark]
	fn annul_course(c: Linear<1, { T::MaxStudentsPerCourse::get() }>) {
		let owner: T::AccountId = account("owner", 0, 0);
		let course_id = open_course::<T>(&owner);
		fill_class::<T>(course_id, &owner, c);
		past_min_duration::<T>();
		assert_ok!(Perwerde::<T>::submit_results(
			RawOrigin::Signed(owner.clone()).into(),
			course_id
		));
		for teacher in make_teachers::<T>(T::RatificationsRequired::get()) {
			assert_ok!(Perwerde::<T>::ratify_results(RawOrigin::Signed(teacher).into(), course_id));
		}
		assert_ok!(Perwerde::<T>::report_course_fraud(RawOrigin::Root.into(), course_id));

		#[extrinsic_call]
		_(RawOrigin::Root, course_id);

		assert_eq!(Courses::<T>::get(course_id).unwrap().status, CourseStatus::Annulled);
	}

	impl_benchmark_test_suite!(Perwerde, crate::mock::new_test_ext(), crate::mock::Test);
}
