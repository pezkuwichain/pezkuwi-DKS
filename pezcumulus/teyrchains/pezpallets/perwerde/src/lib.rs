// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

//! # Perwerde (Education) Pezpallet
//!
//! A pezpallet for managing educational courses, student enrollments, and achievement tracking.
//!
//! ## Overview
//!
//! The Perwerde pezpallet implements an on-chain educational platform where:
//! - Educators create and manage courses with IPFS-linked content
//! - Students enroll in courses and track their progress
//! - Course completion earns points that contribute to trust scores
//! - Educational achievements are permanently recorded on-chain
//!
//! ## Core Features
//!
//! ### Course Management
//! - Admins create courses with name, description, and content links (IPFS)
//! - Courses can be active or archived
//! - Each course has a unique ID and owner
//! - Course metadata is immutable after creation
//!
//! ### Student Enrollment
//! - Students enroll in active courses
//! - One enrollment per student per course
//! - Enrollment history tracked with block numbers
//! - Students can be enrolled in multiple courses simultaneously
//!
//! ### Completion & Points
//! - Course owners mark student completions
//! - Points awarded upon completion
//! - Points contribute to Perwerde score for trust calculation
//! - Completion timestamps recorded permanently
//!
//! ## Perwerde Score System
//!
//! The Perwerde score is derived from total education points:
//! - Each completed course awards points
//! - Points accumulate over time
//! - Score used by `pezpallet-trust` for composite trust calculation
//! - Higher education achievement improves ecosystem standing
//!
//! ## Interface
//!
//! ### Extrinsics
//!
//! - `create_course(name, description, content_link)` - Create new educational course (admin)
//! - `enroll_student(course_id)` - Enroll in an active course (user)
//! - `mark_course_completed(student, course_id, points)` - Award completion points (course owner)
//! - `archive_course(course_id)` - Archive a course (course owner)
//!
//! ### Storage
//!
//! - `Courses` - Course metadata indexed by course ID
//! - `NextCourseId` - Auto-incrementing course ID counter
//! - `Enrollments` - Student enrollment records (student, course_id) → Enrollment
//! - `StudentCourses` - Per-student list of enrolled course IDs
//!
//! ### Integration
//!
//! - Implements `PerwerdeScoreProvider` trait for `pezpallet-trust`
//! - Education scores contribute to validator eligibility
//! - Course completion history visible to governance
//!
//! ## Security Features
//!
//! - Only course owners can mark completions
//! - Active courses required for enrollment
//! - No duplicate enrollments
//! - Maximum courses per student limit
//! - Admin-only course creation
//!
//! ## Runtime Integration Example
//!
//! ```ignore
//! impl pezpallet_perwerde::Config for Runtime {
//!     type RuntimeEvent = RuntimeEvent;
//!     type AdminOrigin = EnsureRoot<AccountId>;
//!     type WeightInfo = pezpallet_perwerde::weights::BizinikiwiWeight<Runtime>;
//!     type MaxCourseNameLength = ConstU32<128>;
//!     type MaxCourseDescLength = ConstU32<512>;
//!     type MaxCourseLinkLength = ConstU32<256>;
//!     type MaxStudentsPerCourse = ConstU32<100>;
//! }
//! ```

pub use pezpallet::*;

/// Trait for notifying trust score system when perwerde score changes.
/// Defined locally to avoid cyclic dependency with pezpallet-trust.
pub trait TrustScoreUpdater<AccountId> {
	fn on_score_component_changed(who: &AccountId);
}

/// Noop implementation for mock environments.
impl<AccountId> TrustScoreUpdater<AccountId> for () {
	fn on_score_component_changed(_who: &AccountId) {}
}

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

// These modules should only be compiled in `std` environment.
#[cfg(all(feature = "std", any(test, feature = "runtime-benchmarks")))]
pub mod mock;

#[cfg(all(feature = "std", test))]
mod tests;

pub use weights::WeightInfo;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::{
		dispatch::DispatchResult,
		pezpallet_prelude::*,
		traits::{EnsureOrigin, Get},
	};
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config<RuntimeEvent: From<Event<Self>>> {
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
		type WeightInfo: WeightInfo;

		#[pezpallet::constant]
		type MaxCourseNameLength: Get<u32>;
		#[pezpallet::constant]
		type MaxCourseDescLength: Get<u32>;
		#[pezpallet::constant]
		type MaxCourseLinkLength: Get<u32>;
		#[pezpallet::constant]
		type MaxStudentsPerCourse: Get<u32>;

		/// Maximum number of courses a single student can enroll in
		/// Used for StudentCourses storage bound
		#[pezpallet::constant]
		type MaxCoursesPerStudent: Get<u32>;

		/// Maximum points that can be awarded per course completion.
		/// Prevents unbounded point inflation by course owners.
		#[pezpallet::constant]
		type MaxPointsPerCourse: Get<u32>;

		/// Trust score updater - notifies trust pallet when perwerde score changes
		type TrustScoreUpdater: TrustScoreUpdater<Self::AccountId>;
	}

	#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum CourseStatus {
		Active,
		Archived,
	}

	#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Course<T: Config> {
		pub id: u32,
		pub owner: T::AccountId,
		pub name: BoundedVec<u8, T::MaxCourseNameLength>,
		pub description: BoundedVec<u8, T::MaxCourseDescLength>,
		pub content_link: BoundedVec<u8, T::MaxCourseLinkLength>,
		pub status: CourseStatus,
		pub created_at: BlockNumberFor<T>,
	}

	#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Enrollment<T: Config> {
		pub student: T::AccountId,
		pub course_id: u32,
		pub enrolled_at: BlockNumberFor<T>,
		pub completed_at: Option<BlockNumberFor<T>>,
		pub points_earned: u32,
	}

	#[pezpallet::storage]
	#[pezpallet::getter(fn courses)]
	pub type Courses<T: Config> = StorageMap<_, Blake2_128Concat, u32, Course<T>, OptionQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn next_course_id)]
	pub type NextCourseId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn enrollments)]
	pub type Enrollments<T: Config> =
		StorageMap<_, Blake2_128Concat, (T::AccountId, u32), Enrollment<T>, OptionQuery>;

	/// Per-student list of enrolled course IDs
	/// UPDATED (Gemini suggestion): Uses MaxCoursesPerStudent instead of MaxStudentsPerCourse
	/// This is the correct semantic - limits how many courses ONE student can take
	#[pezpallet::storage]
	#[pezpallet::getter(fn student_courses)]
	pub type StudentCourses<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<u32, T::MaxCoursesPerStudent>,
		ValueQuery,
	>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CourseCreated { course_id: u32, owner: T::AccountId },
		StudentEnrolled { student: T::AccountId, course_id: u32 },
		CourseCompleted { student: T::AccountId, course_id: u32, points: u32 },
		CourseArchived { course_id: u32 },
	}

	#[pezpallet::error]
	pub enum Error<T> {
		CourseNotFound,
		AlreadyEnrolled,
		NotEnrolled,
		CourseNotActive,
		CourseAlreadyCompleted,
		NotCourseOwner,
		TooManyCourses,
		/// Course ID counter overflow
		CourseIdOverflow,
		/// Points exceed the maximum allowed per course
		PointsExceedMax,
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::create_course())]
		pub fn create_course(
			origin: OriginFor<T>,
			name: BoundedVec<u8, T::MaxCourseNameLength>,
			description: BoundedVec<u8, T::MaxCourseDescLength>,
			content_link: BoundedVec<u8, T::MaxCourseLinkLength>,
		) -> DispatchResult {
			let owner = T::AdminOrigin::ensure_origin(origin)?;
			let course_id = NextCourseId::<T>::get();

			// Prevent overflow — ensure we haven't exhausted the u32 ID space
			ensure!(course_id < u32::MAX, Error::<T>::CourseIdOverflow);

			let course = Course {
				id: course_id,
				owner: owner.clone(),
				name,
				description,
				content_link,
				status: CourseStatus::Active,
				created_at: pezframe_system::Pezpallet::<T>::block_number(),
			};

			Courses::<T>::insert(course_id, course);
			NextCourseId::<T>::put(course_id.saturating_add(1));

			Self::deposit_event(Event::CourseCreated { course_id, owner });
			Ok(())
		}

		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::enroll())]
		pub fn enroll(origin: OriginFor<T>, course_id: u32) -> DispatchResult {
			let student = ensure_signed(origin)?;
			let course = Courses::<T>::get(course_id).ok_or(Error::<T>::CourseNotFound)?;
			ensure!(course.status == CourseStatus::Active, Error::<T>::CourseNotActive);
			ensure!(
				!Enrollments::<T>::contains_key((&student, course_id)),
				Error::<T>::AlreadyEnrolled
			);

			let enrollment = Enrollment {
				student: student.clone(),
				course_id,
				enrolled_at: pezframe_system::Pezpallet::<T>::block_number(),
				completed_at: None,
				points_earned: 0,
			};

			Enrollments::<T>::insert((&student, course_id), enrollment);
			StudentCourses::<T>::try_mutate(&student, |courses| {
				courses.try_push(course_id).map_err(|_| Error::<T>::TooManyCourses)
			})?;

			Self::deposit_event(Event::StudentEnrolled { student, course_id });
			Ok(())
		}

		/// Mark a student's course as completed and award points
		/// SECURITY: Only the course owner can mark completions, not students themselves
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::complete_course())]
		pub fn complete_course(
			origin: OriginFor<T>,
			student: T::AccountId,
			course_id: u32,
			points: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// Validate points are within the allowed maximum
			ensure!(points <= T::MaxPointsPerCourse::get(), Error::<T>::PointsExceedMax);

			// Verify caller is the course owner
			let course = Courses::<T>::get(course_id).ok_or(Error::<T>::CourseNotFound)?;
			ensure!(course.owner == caller, Error::<T>::NotCourseOwner);

			// Get and validate enrollment
			let mut enrollment =
				Enrollments::<T>::get((&student, course_id)).ok_or(Error::<T>::NotEnrolled)?;
			ensure!(enrollment.completed_at.is_none(), Error::<T>::CourseAlreadyCompleted);

			// Mark completion
			enrollment.completed_at = Some(pezframe_system::Pezpallet::<T>::block_number());
			enrollment.points_earned = points;

			Enrollments::<T>::insert((&student, course_id), enrollment);

			Self::deposit_event(Event::CourseCompleted {
				student: student.clone(),
				course_id,
				points,
			});

			// Notify trust pallet that student's perwerde score component changed
			T::TrustScoreUpdater::on_score_component_changed(&student);

			Ok(())
		}

		#[pezpallet::call_index(3)]
		#[pezpallet::weight(T::WeightInfo::archive_course())]
		pub fn archive_course(origin: OriginFor<T>, course_id: u32) -> DispatchResult {
			let caller = T::AdminOrigin::ensure_origin(origin)?;
			let mut course = Courses::<T>::get(course_id).ok_or(Error::<T>::CourseNotFound)?;
			ensure!(course.owner == caller, Error::<T>::NotCourseOwner);

			course.status = CourseStatus::Archived;
			Courses::<T>::insert(course_id, course);

			Self::deposit_event(Event::CourseArchived { course_id });
			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		pub fn get_perwerde_score(who: &T::AccountId) -> u32 {
			StudentCourses::<T>::get(who)
				.iter()
				.filter_map(|course_id| Enrollments::<T>::get((who, *course_id)))
				.filter(|enrollment| enrollment.completed_at.is_some())
				.map(|enrollment| enrollment.points_earned)
				.fold(0u32, |acc, points| acc.saturating_add(points))
		}
	}
}
