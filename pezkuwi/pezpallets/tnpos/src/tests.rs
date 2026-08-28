// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{mock::*, *};
use pezframe_support::assert_ok;

#[test]
fn genesis_installs_the_nine_strata() {
	new_test_ext().execute_with(|| {
		assert_eq!(Strata::<Test>::get().len(), 9);
		assert_eq!(CurrentEra::<Test>::get(), 0);
		assert!(CurrentCommittee::<Test>::get().is_empty());
	});
}

#[test]
fn genesis_refuses_a_configuration_that_cannot_be_seated() {
	// Four strata cannot clear MIN_STRATA, so building genesis with them must fail rather
	// than start a chain whose committee is below the security budget from block zero.
	assert!(std::panic::catch_unwind(|| { new_test_ext_with_strata(4) }).is_err());
}

#[test]
fn set_strata_requires_the_manager_origin() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tnpos::set_strata(RuntimeOrigin::root(), nine_strata()));
	});
}
