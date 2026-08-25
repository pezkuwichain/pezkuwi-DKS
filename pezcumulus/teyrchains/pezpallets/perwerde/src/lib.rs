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
//! - `Enrollments` - Enrolment records, keyed (course_id, student) so a class reads by prefix
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

extern crate alloc;

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
	use pezpallet_tiki::{EarnedRoleGranter, Tiki, TikiProvider};
	use pezsp_runtime::traits::Saturating;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		/// What the education record claims has to match what was actually awarded.
		///
		/// The score is kept rather than recomputed, which is what makes it affordable and
		/// what makes it capable of drifting. It is worth 300 in the trust formula and trust
		/// is candidacy for office, so a score that quietly ran ahead of the courses behind it
		/// would be standing invented out of nothing -- and it would look exactly like
		/// standing that had been earned.
		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use alloc::collections::BTreeMap;
			use pezframe_support::ensure;

			let mut awarded: BTreeMap<T::AccountId, u32> = BTreeMap::new();
			let mut rewarded: BTreeMap<T::AccountId, u32> = BTreeMap::new();

			for (_, student, enrollment) in Enrollments::<T>::iter() {
				if enrollment.points_awarded > 0 {
					*awarded.entry(student.clone()).or_default() += enrollment.points_awarded;
					*rewarded.entry(student).or_default() += 1;
				}
			}

			for (student, score) in PerwerdeScores::<T>::iter() {
				ensure!(
					awarded.get(&student).copied().unwrap_or(0) == score,
					"a student's score does not match what their courses awarded"
				);
			}

			for (student, count) in RewardedCourses::<T>::iter() {
				ensure!(
					rewarded.get(&student).copied().unwrap_or(0) == count,
					"a student's rewarded-course count does not match their awards"
				);
				ensure!(
					count <= T::RewardedCourseLimit::get(),
					"a student was rewarded for more courses than the limit allows"
				);
				// Every rewarded course is a completed one; the reverse need not hold, since
				// study past the limit still completes.
				ensure!(
					count <= CompletedCourses::<T>::get(&student),
					"more of a student's courses were rewarded than they completed"
				);
			}

			// A course cannot be closed on fewer signatures than the board requires, and the
			// teacher is never one of them.
			for (course_id, course) in Courses::<T>::iter() {
				if course.status == CourseStatus::Completed {
					ensure!(
						RatificationCount::<T>::get(course_id) >= T::RatificationsRequired::get(),
						"a course was closed without a full board"
					);
				}
				ensure!(
					!CourseRatifiers::<T>::contains_key(course_id, &course.owner),
					"a teacher ratified their own course"
				);
			}

			ensure!(
				HonoraryMamosteCount::<T>::get() <= T::MaxHonoraryMamoste::get(),
				"more teachers were seeded than the minister may seed"
			);

			Ok(())
		}
	}

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config<RuntimeEvent: From<Event<Self>>> {
		/// How the pallet asks who holds which role.
		///
		/// A provider rather than a supertrait on the tiki pallet: this pallet needs one
		/// question answered -- is this account a teacher -- and taking the whole config would
		/// drag the NFT and citizenship machinery in behind it, including into every test.
		type TikiSource: pezpallet_tiki::TikiProvider<Self::AccountId>;

		/// Who may open a course.
		///
		/// Deliberately narrow, the way an education council is: opening a course decides what
		/// the state will recognise as learning, and the points behind it become political
		/// standing. Root and the council resolve to a keyless account here, which is why the
		/// owner is named separately.
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

		/// The education minister: `WezireBelaw`.
		///
		/// Seeds the examiner corps and brings fraud to the court. Not a grader and not a
		/// ratifier -- the minister decides what is taught, not who passed.
		type EducationMinisterOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Who annuls a fraudulent course. The court.
		type FraudOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// How an earned role is awarded once the record supports it.
		type EarnedRoles: EarnedRoleGranter<Self::AccountId, Tiki>;

		type WeightInfo: WeightInfo;

		#[pezpallet::constant]
		type MaxCourseNameLength: Get<u32>;
		#[pezpallet::constant]
		type MaxCourseDescLength: Get<u32>;
		#[pezpallet::constant]
		type MaxCourseLinkLength: Get<u32>;
		#[pezpallet::constant]
		type MaxStudentsPerCourse: Get<u32>;

		/// The most a single course may be worth.
		///
		/// The value of a course is fixed when it is opened, not chosen for each student when
		/// they are graded. A teacher who could pick the number for every student could give
		/// a friend ten times what the work was worth, and the difference would be trust score
		/// -- which is candidacy for office.
		#[pezpallet::constant]
		type MaxPointsPerCourse: Get<u32>;

		/// The shortest a course may run before its results can be ratified.
		///
		/// A course that could be opened and closed in an afternoon is a way of printing
		/// standing, not a way of teaching.
		#[pezpallet::constant]
		type MinCourseDuration: Get<BlockNumberFor<Self>>;

		/// The longest a course may stay open. After this it expires and awards nothing.
		#[pezpallet::constant]
		type MaxCourseDuration: Get<BlockNumberFor<Self>>;

		/// How many teachers must ratify a course's results.
		///
		/// Grading is where the power is, so it is not one person's. The board ratifies the
		/// whole cohort at once rather than each student -- one submission and a handful of
		/// approvals, instead of five signatures per student in a class of a thousand.
		#[pezpallet::constant]
		type RatificationsRequired: Get<u32>;

		/// How many teachers the minister may appoint directly.
		///
		/// Without this the pallet cannot start: ratifying a course needs teachers, and being
		/// a teacher is earned by completing courses. The cap is what keeps a bootstrap from
		/// becoming a standing power to hand out credentials.
		#[pezpallet::constant]
		type MaxHonoraryMamoste: Get<u32>;

		/// How many completed courses count towards standing.
		///
		/// Beyond this a citizen may keep studying -- and should -- but the score stops
		/// growing. Learning is not capped; what it is worth in political weight is.
		#[pezpallet::constant]
		type RewardedCourseLimit: Get<u32>;

		/// Completed courses required before any earned role is considered.
		#[pezpallet::constant]
		type MinCoursesForRole: Get<u32>;

		/// Points thresholds for the roles this pallet awards.
		#[pezpallet::constant]
		type RewsenbirThreshold: Get<u32>;
		#[pezpallet::constant]
		type MamosteThreshold: Get<u32>;
		#[pezpallet::constant]
		type AxaThreshold: Get<u32>;

		/// Trust score updater - notifies trust pallet when perwerde score changes
		type TrustScoreUpdater: TrustScoreUpdater<Self::AccountId>;
	}

	/// Where a course is in its life.
	#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
	pub enum CourseStatus {
		/// Open: students may enrol and the teacher may record results, which are drafts.
		Enrolling,
		/// The teacher has submitted the results; the board is ratifying.
		AwaitingRatification,
		/// Ratified. The points are real and the course is closed.
		Completed,
		/// The year ran out before the results were ratified. Nothing was awarded.
		Expired,
		/// The court found the course fraudulent. What it awarded has been taken back.
		Annulled,
	}

	#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Course<T: Config> {
		pub id: u32,
		/// Who teaches it and records the results. Named by whoever opened the course, because
		/// the privileged origins resolve to a keyless account that can sign nothing.
		pub owner: T::AccountId,
		pub name: BoundedVec<u8, T::MaxCourseNameLength>,
		pub description: BoundedVec<u8, T::MaxCourseDescLength>,
		pub content_link: BoundedVec<u8, T::MaxCourseLinkLength>,
		/// What the course is worth, decided when it is opened.
		pub points: u32,
		pub status: CourseStatus,
		pub created_at: BlockNumberFor<T>,
		/// When the results were submitted for ratification.
		pub submitted_at: Option<BlockNumberFor<T>>,
	}

	#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Enrollment<T: Config> {
		pub student: T::AccountId,
		pub course_id: u32,
		pub enrolled_at: BlockNumberFor<T>,
		/// What the teacher recorded. A draft until the board ratifies: recording it changes
		/// no score and can be changed again while the course is open.
		pub passed: Option<bool>,
		/// What the student was actually awarded, once the results were ratified.
		pub points_awarded: u32,
	}

	#[pezpallet::storage]
	#[pezpallet::getter(fn courses)]
	pub type Courses<T: Config> = StorageMap<_, Blake2_128Concat, u32, Course<T>, OptionQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn next_course_id)]
	pub type NextCourseId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn course_enrollment_count)]
	pub type CourseEnrollmentCount<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, u32, ValueQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn enrollments)]
	/// Keyed by course first, so a class can be read with a prefix.
	///
	/// It used to be one map under a `(student, course)` tuple, which cannot be prefixed --
	/// so every place that needed "the students on this course" walked the whole register and
	/// filtered in memory. That is not a fraud-path detail: `close_and_award` runs on every
	/// course that finishes, and it read every enrolment on the chain to find its own hundred.
	pub type Enrollments<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32, // course
		Blake2_128Concat,
		T::AccountId, // student
		Enrollment<T>,
		OptionQuery,
	>;

	/// Which teachers have ratified a course's results.
	#[pezpallet::storage]
	#[pezpallet::getter(fn course_ratifier)]
	pub type CourseRatifiers<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, T::AccountId, (), OptionQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn ratification_count)]
	pub type RatificationCount<T: Config> = StorageMap<_, Blake2_128Concat, u32, u32, ValueQuery>;

	/// What each student has earned, kept rather than recomputed.
	///
	/// This used to be a walk of every course a student had ever enrolled in, which is why the
	/// number of courses one person could take was bounded at all -- the bound existed to keep
	/// the sum cheap, and it locked anyone who reached it out of studying for the rest of
	/// their life. A running total has no such cost and no such limit.
	#[pezpallet::storage]
	#[pezpallet::getter(fn perwerde_score)]
	pub type PerwerdeScores<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	/// How many completed courses have already counted towards a student's score.
	#[pezpallet::storage]
	#[pezpallet::getter(fn rewarded_courses)]
	pub type RewardedCourses<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	/// How many courses each student has completed, rewarded or not.
	#[pezpallet::storage]
	#[pezpallet::getter(fn completed_courses)]
	pub type CompletedCourses<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	/// Teachers the minister appointed rather than teachers who earned it.
	///
	/// The same tiki and the same standing -- a teacher is a teacher. Recorded separately for
	/// the same reason honorary citizenship is: so the chain can be asked how much of the
	/// examiner corps was seeded to get started and how much of it taught its way there.
	#[pezpallet::storage]
	#[pezpallet::getter(fn is_honorary_mamoste)]
	pub type HonoraryMamoste<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn honorary_mamoste_count)]
	pub type HonoraryMamosteCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Courses the minister has brought before the court.
	#[pezpallet::storage]
	#[pezpallet::getter(fn course_under_review)]
	pub type CoursesUnderReview<T: Config> = StorageMap<_, Blake2_128Concat, u32, (), OptionQuery>;

	/// Ratifications of courses the court later annulled, per teacher.
	///
	/// Ratifying is vouching: five teachers saying these results are real. If the court finds
	/// they were not, that is on the board as much as on whoever wrote them. Kept rather than
	/// punished automatically -- what it costs a teacher is a matter for the court and the
	/// ministry, and the record is what they decide on.
	#[pezpallet::storage]
	#[pezpallet::getter(fn annulled_ratifications)]
	pub type AnnulledRatifications<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CourseCreated {
			course_id: u32,
			owner: T::AccountId,
			points: u32,
		},
		StudentEnrolled {
			student: T::AccountId,
			course_id: u32,
		},
		/// The teacher recorded a result. It counts for nothing until the board ratifies.
		ResultRecorded {
			student: T::AccountId,
			course_id: u32,
			passed: bool,
		},
		ResultsSubmitted {
			course_id: u32,
		},
		CourseRatified {
			course_id: u32,
			by: T::AccountId,
			ratifications: u32,
		},
		/// The board is complete, the course is closed and the points are real.
		CourseCompleted {
			course_id: u32,
			awarded_to: u32,
		},
		/// The year ran out with the results unratified.
		CourseExpired {
			course_id: u32,
		},
		/// The minister has brought a course before the court.
		CourseReported {
			course_id: u32,
		},
		/// The court annulled a course; what it awarded has been taken back.
		CourseAnnulled {
			course_id: u32,
			points_reversed: u32,
		},
		/// The minister seeded a teacher.
		HonoraryMamosteAppointed {
			who: T::AccountId,
		},
	}

	#[pezpallet::error]
	pub enum Error<T> {
		CourseNotFound,
		AlreadyEnrolled,
		NotEnrolled,
		CourseNotActive,
		NotCourseOwner,
		CourseIdOverflow,
		PointsExceedMax,
		TooManyStudents,
		/// A course cannot be taught to the person teaching it.
		OwnerCannotEnrol,
		/// The course has not run long enough for its results to be ratified.
		TooEarlyToSubmit,
		/// The course is not waiting for ratification.
		NotAwaitingRatification,
		/// A teacher cannot ratify their own course.
		OwnerCannotRatify,
		/// Only a teacher may ratify results.
		NotATeacher,
		/// This teacher has already ratified this course.
		AlreadyRatified,
		/// The course still has time to run.
		NotYetExpired,
		/// The examiner corps is already at the size the minister may seed.
		HonoraryMamosteLimitReached,
		/// The court has not been asked to look at this course.
		NotUnderReview,
		/// A course that awarded nothing cannot be annulled.
		NothingToAnnul,
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Open a course, and say what it is worth.
		///
		/// The owner is named rather than taken from the origin. Root and the council resolve
		/// to a keyless account, so a course they opened had a teacher who could not sign
		/// anything and therefore could never grade anyone -- which meant, in practice, that
		/// only courses opened by the President could ever be completed.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::create_course())]
		pub fn create_course(
			origin: OriginFor<T>,
			owner: T::AccountId,
			name: BoundedVec<u8, T::MaxCourseNameLength>,
			description: BoundedVec<u8, T::MaxCourseDescLength>,
			content_link: BoundedVec<u8, T::MaxCourseLinkLength>,
			points: u32,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			ensure!(points <= T::MaxPointsPerCourse::get(), Error::<T>::PointsExceedMax);

			let course_id = NextCourseId::<T>::get();
			ensure!(course_id < u32::MAX, Error::<T>::CourseIdOverflow);

			Courses::<T>::insert(
				course_id,
				Course {
					id: course_id,
					owner: owner.clone(),
					name,
					description,
					content_link,
					points,
					status: CourseStatus::Enrolling,
					created_at: pezframe_system::Pezpallet::<T>::block_number(),
					submitted_at: None,
				},
			);
			NextCourseId::<T>::put(course_id.saturating_add(1));

			Self::deposit_event(Event::CourseCreated { course_id, owner, points });
			Ok(())
		}

		/// Enrol in a course.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::enroll())]
		pub fn enroll(origin: OriginFor<T>, course_id: u32) -> DispatchResult {
			let student = ensure_signed(origin)?;
			let course = Courses::<T>::get(course_id).ok_or(Error::<T>::CourseNotFound)?;
			ensure!(course.status == CourseStatus::Enrolling, Error::<T>::CourseNotActive);
			// Teaching yourself is not a credential. Without this the teacher could enrol in
			// their own course and award themselves whatever it was worth, as many times as
			// they cared to open one.
			ensure!(student != course.owner, Error::<T>::OwnerCannotEnrol);
			ensure!(
				!Enrollments::<T>::contains_key(course_id, &student),
				Error::<T>::AlreadyEnrolled
			);

			let enrolled = CourseEnrollmentCount::<T>::get(course_id);
			ensure!(enrolled < T::MaxStudentsPerCourse::get(), Error::<T>::TooManyStudents);

			Enrollments::<T>::insert(
				course_id,
				&student,
				Enrollment {
					student: student.clone(),
					course_id,
					enrolled_at: pezframe_system::Pezpallet::<T>::block_number(),
					passed: None,
					points_awarded: 0,
				},
			);
			CourseEnrollmentCount::<T>::insert(course_id, enrolled.saturating_add(1));

			Self::deposit_event(Event::StudentEnrolled { student, course_id });
			Ok(())
		}

		/// Record how a student did. A draft, and changeable, until the board ratifies.
		///
		/// Pass or fail, and nothing in between: what the course is worth was decided when it
		/// was opened. The teacher who used to choose the number for each student could give
		/// one person ten times what another got for the same work.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::record_result())]
		pub fn record_result(
			origin: OriginFor<T>,
			student: T::AccountId,
			course_id: u32,
			passed: bool,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let course = Courses::<T>::get(course_id).ok_or(Error::<T>::CourseNotFound)?;
			ensure!(course.owner == caller, Error::<T>::NotCourseOwner);
			ensure!(course.status == CourseStatus::Enrolling, Error::<T>::CourseNotActive);

			Enrollments::<T>::try_mutate(course_id, &student, |slot| -> DispatchResult {
				let enrollment = slot.as_mut().ok_or(Error::<T>::NotEnrolled)?;
				enrollment.passed = Some(passed);
				Ok(())
			})?;

			Self::deposit_event(Event::ResultRecorded { student, course_id, passed });
			Ok(())
		}

		/// Submit the course's results to the examining board.
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(T::WeightInfo::submit_results())]
		pub fn submit_results(origin: OriginFor<T>, course_id: u32) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let now = pezframe_system::Pezpallet::<T>::block_number();

			Courses::<T>::try_mutate(course_id, |slot| -> DispatchResult {
				let course = slot.as_mut().ok_or(Error::<T>::CourseNotFound)?;
				ensure!(course.owner == caller, Error::<T>::NotCourseOwner);
				ensure!(course.status == CourseStatus::Enrolling, Error::<T>::CourseNotActive);
				ensure!(
					now >= course.created_at.saturating_add(T::MinCourseDuration::get()),
					Error::<T>::TooEarlyToSubmit
				);
				course.status = CourseStatus::AwaitingRatification;
				course.submitted_at = Some(now);
				Ok(())
			})?;

			Self::deposit_event(Event::ResultsSubmitted { course_id });
			Ok(())
		}

		/// Ratify a course's results. A teacher's act, and never the teacher's own course.
		///
		/// When enough of the board has ratified, the course closes and every recorded pass
		/// becomes points. Before that nothing the teacher wrote has any effect at all.
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(T::WeightInfo::ratify_results(T::MaxStudentsPerCourse::get()))]
		pub fn ratify_results(origin: OriginFor<T>, course_id: u32) -> DispatchResult {
			let ratifier = ensure_signed(origin)?;
			let course = Courses::<T>::get(course_id).ok_or(Error::<T>::CourseNotFound)?;
			ensure!(
				course.status == CourseStatus::AwaitingRatification,
				Error::<T>::NotAwaitingRatification
			);
			ensure!(course.owner != ratifier, Error::<T>::OwnerCannotRatify);
			ensure!(T::TikiSource::has_tiki(&ratifier, &Tiki::Mamoste), Error::<T>::NotATeacher);
			ensure!(
				!CourseRatifiers::<T>::contains_key(course_id, &ratifier),
				Error::<T>::AlreadyRatified
			);

			CourseRatifiers::<T>::insert(course_id, &ratifier, ());
			let count = RatificationCount::<T>::mutate(course_id, |n| {
				*n = n.saturating_add(1);
				*n
			});
			Self::deposit_event(Event::CourseRatified {
				course_id,
				by: ratifier,
				ratifications: count,
			});

			if count >= T::RatificationsRequired::get() {
				Self::close_and_award(course_id)?;
			}
			Ok(())
		}

		/// Close a course whose year ran out without its results being ratified.
		///
		/// Permissionless: it decides nothing, it only records what the calendar already
		/// settled. An expired course awards nothing, which is the point -- results that
		/// nobody would put their name to do not become standing by being left alone.
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(T::WeightInfo::expire_course())]
		pub fn expire_course(origin: OriginFor<T>, course_id: u32) -> DispatchResult {
			ensure_signed(origin)?;
			let now = pezframe_system::Pezpallet::<T>::block_number();

			Courses::<T>::try_mutate(course_id, |slot| -> DispatchResult {
				let course = slot.as_mut().ok_or(Error::<T>::CourseNotFound)?;
				ensure!(
					matches!(
						course.status,
						CourseStatus::Enrolling | CourseStatus::AwaitingRatification
					),
					Error::<T>::CourseNotActive
				);
				ensure!(
					now > course.created_at.saturating_add(T::MaxCourseDuration::get()),
					Error::<T>::NotYetExpired
				);
				course.status = CourseStatus::Expired;
				Ok(())
			})?;

			Self::deposit_event(Event::CourseExpired { course_id });
			Ok(())
		}

		/// Seed a teacher.
		///
		/// The minister's, and capped. Ratifying a course needs teachers and being a teacher
		/// is earned by completing courses, so without a way to seed the first ones the pallet
		/// would start locked: no course could ever be ratified, so nobody could ever earn the
		/// role that ratifies. The cap is what stops a bootstrap from becoming a standing
		/// power to hand out credentials.
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(T::WeightInfo::appoint_honorary_mamoste())]
		pub fn appoint_honorary_mamoste(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			T::EducationMinisterOrigin::ensure_origin(origin)?;
			ensure!(
				HonoraryMamosteCount::<T>::get() < T::MaxHonoraryMamoste::get(),
				Error::<T>::HonoraryMamosteLimitReached
			);

			T::EarnedRoles::grant_earned(&who, Tiki::Mamoste)?;
			HonoraryMamoste::<T>::insert(&who, ());
			HonoraryMamosteCount::<T>::mutate(|n| *n = n.saturating_add(1));

			Self::deposit_event(Event::HonoraryMamosteAppointed { who });
			Ok(())
		}

		/// Bring a course before the court.
		#[pezpallet::call_index(7)]
		#[pezpallet::weight(T::WeightInfo::report_course_fraud())]
		pub fn report_course_fraud(origin: OriginFor<T>, course_id: u32) -> DispatchResult {
			T::EducationMinisterOrigin::ensure_origin(origin)?;
			ensure!(Courses::<T>::contains_key(course_id), Error::<T>::CourseNotFound);

			CoursesUnderReview::<T>::insert(course_id, ());
			Self::deposit_event(Event::CourseReported { course_id });
			Ok(())
		}

		/// Annul a course the court has found fraudulent, and take back what it awarded.
		///
		/// Everything else in the state can be undone by the court -- an office, a
		/// citizenship -- and a credential should be no different. Standing bought from a
		/// teacher is standing all the same until somebody can remove it.
		#[pezpallet::call_index(8)]
		#[pezpallet::weight(T::WeightInfo::annul_course(T::MaxStudentsPerCourse::get()))]
		pub fn annul_course(origin: OriginFor<T>, course_id: u32) -> DispatchResult {
			T::FraudOrigin::ensure_origin(origin)?;
			ensure!(CoursesUnderReview::<T>::contains_key(course_id), Error::<T>::NotUnderReview);

			let mut course = Courses::<T>::get(course_id).ok_or(Error::<T>::CourseNotFound)?;
			ensure!(course.status == CourseStatus::Completed, Error::<T>::NothingToAnnul);

			// Take the points back from everyone the course awarded. A prefix read, so the
			// cost is the class -- bounded at enrolment by `MaxStudentsPerCourse` -- and not
			// the register.
			let mut reversed = 0u32;
			for (student, mut enrollment) in Enrollments::<T>::iter_prefix(course_id) {
				if enrollment.points_awarded > 0 {
					let awarded = enrollment.points_awarded;
					PerwerdeScores::<T>::mutate(&student, |s| *s = s.saturating_sub(awarded));
					RewardedCourses::<T>::mutate(&student, |n| *n = n.saturating_sub(1));
					CompletedCourses::<T>::mutate(&student, |n| *n = n.saturating_sub(1));
					reversed = reversed.saturating_add(awarded);
					enrollment.points_awarded = 0;
					Enrollments::<T>::insert(course_id, &student, enrollment);
					T::TrustScoreUpdater::on_score_component_changed(&student);
				}
			}

			// The board vouched for these results. What that costs them is the court's and the
			// ministry's to decide; the record is what they decide on.
			for (ratifier, _) in CourseRatifiers::<T>::iter_prefix(course_id) {
				AnnulledRatifications::<T>::mutate(&ratifier, |n| *n = n.saturating_add(1));
			}

			course.status = CourseStatus::Annulled;
			Courses::<T>::insert(course_id, course);
			CoursesUnderReview::<T>::remove(course_id);

			Self::deposit_event(Event::CourseAnnulled { course_id, points_reversed: reversed });
			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		/// Close a ratified course and turn its recorded passes into points.
		fn close_and_award(course_id: u32) -> DispatchResult {
			let course = Courses::<T>::get(course_id).ok_or(Error::<T>::CourseNotFound)?;
			let mut awarded_to = 0u32;

			for (student, mut enrollment) in Enrollments::<T>::iter_prefix(course_id) {
				if enrollment.passed != Some(true) {
					continue;
				}

				CompletedCourses::<T>::mutate(&student, |n| *n = n.saturating_add(1));

				// Beyond the limit a citizen may keep studying, and the record keeps saying
				// so; what stops growing is the political weight of it.
				let rewarded = RewardedCourses::<T>::get(&student);
				if rewarded < T::RewardedCourseLimit::get() {
					RewardedCourses::<T>::insert(&student, rewarded.saturating_add(1));
					PerwerdeScores::<T>::mutate(&student, |s| *s = s.saturating_add(course.points));
					enrollment.points_awarded = course.points;
					Enrollments::<T>::insert(course_id, &student, enrollment);
					T::TrustScoreUpdater::on_score_component_changed(&student);
				}

				Self::award_earned_roles(&student);
				awarded_to = awarded_to.saturating_add(1);
			}

			Courses::<T>::mutate(course_id, |slot| {
				if let Some(course) = slot {
					course.status = CourseStatus::Completed;
				}
			});

			Self::deposit_event(Event::CourseCompleted { course_id, awarded_to });
			Ok(())
		}

		/// Award the roles a student's record now supports.
		///
		/// A points threshold alone would let one enormous course carry somebody to a title,
		/// so a count of completed courses gates all three: an education is several things
		/// learned, not one.
		fn award_earned_roles(who: &T::AccountId) {
			if CompletedCourses::<T>::get(who) < T::MinCoursesForRole::get() {
				return;
			}
			let score = PerwerdeScores::<T>::get(who);

			for (threshold, tiki) in [
				(T::AxaThreshold::get(), Tiki::Axa),
				(T::MamosteThreshold::get(), Tiki::Mamoste),
				(T::RewsenbirThreshold::get(), Tiki::Rewsenbîr),
			] {
				if threshold > 0 && score >= threshold {
					if let Err(e) = T::EarnedRoles::grant_earned(who, tiki) {
						log::warn!(
							target: "perwerde",
							"could not award an earned role to {who:?}: {e:?}"
						);
					}
				}
			}
		}

		/// What a student's education is worth, for the trust score.
		pub fn get_perwerde_score(who: &T::AccountId) -> u32 {
			PerwerdeScores::<T>::get(who)
		}
	}
}
