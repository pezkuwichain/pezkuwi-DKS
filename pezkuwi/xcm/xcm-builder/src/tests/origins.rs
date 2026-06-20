// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

use super::*;

#[test]
fn universal_origin_should_work() {
	AllowUnpaidFrom::set(vec![[Teyrchain(1)].into(), [Teyrchain(2)].into()]);
	clear_universal_aliases();
	// Teyrchain 1 may represent Dicle to us
	add_universal_alias(Teyrchain(1), Dicle);
	// Teyrchain 2 may represent Pezkuwi to us
	add_universal_alias(Teyrchain(2), Pezkuwi);

	let message = Xcm(vec![
		UniversalOrigin(GlobalConsensus(Dicle)),
		TransferAsset { assets: (Parent, 100u128).into(), beneficiary: Here.into() },
	]);
	let mut hash = fake_message_hash(&message);
	let r = XcmExecutor::<TestConfig>::prepare_and_execute(
		Teyrchain(2),
		message,
		&mut hash,
		Weight::from_parts(50, 50),
		Weight::zero(),
	);
	assert_eq!(
		r,
		Outcome::Incomplete {
			used: Weight::from_parts(10, 10),
			error: InstructionError { index: 0, error: XcmError::InvalidLocation },
		}
	);

	let message = Xcm(vec![
		UniversalOrigin(GlobalConsensus(Dicle)),
		TransferAsset { assets: (Parent, 100u128).into(), beneficiary: Here.into() },
	]);
	let mut hash = fake_message_hash(&message);
	let r = XcmExecutor::<TestConfig>::prepare_and_execute(
		Teyrchain(1),
		message,
		&mut hash,
		Weight::from_parts(50, 50),
		Weight::zero(),
	);
	assert_eq!(
		r,
		Outcome::Incomplete {
			used: Weight::from_parts(20, 20),
			error: InstructionError { index: 1, error: XcmError::NotWithdrawable },
		}
	);

	add_asset((Ancestor(2), GlobalConsensus(Dicle)), (Parent, 100));
	let message = Xcm(vec![
		UniversalOrigin(GlobalConsensus(Dicle)),
		TransferAsset { assets: (Parent, 100u128).into(), beneficiary: Here.into() },
	]);
	let mut hash = fake_message_hash(&message);
	let r = XcmExecutor::<TestConfig>::prepare_and_execute(
		Teyrchain(1),
		message,
		&mut hash,
		Weight::from_parts(50, 50),
		Weight::zero(),
	);
	assert_eq!(r, Outcome::Complete { used: Weight::from_parts(20, 20) });
	assert_eq!(asset_list((Ancestor(2), GlobalConsensus(Dicle))), vec![]);
}

#[test]
fn export_message_should_work() {
	// Bridge chain (assumed to be Relay) lets Teyrchain #1 have message execution for free.
	AllowUnpaidFrom::set(vec![[Teyrchain(1)].into()]);
	// Local teyrchain #1 issues a transfer asset on Pezkuwi Relay-chain, transferring 100 Planck
	// to Pezkuwi teyrchain #2.
	let expected_message = Xcm(vec![TransferAsset {
		assets: (Here, 100u128).into(),
		beneficiary: Teyrchain(2).into(),
	}]);
	let expected_hash = fake_message_hash(&expected_message);
	let message = Xcm(vec![ExportMessage {
		network: Pezkuwi,
		destination: Here,
		xcm: expected_message.clone(),
	}]);
	let mut hash = fake_message_hash(&message);
	let r = XcmExecutor::<TestConfig>::prepare_and_execute(
		Teyrchain(1),
		message,
		&mut hash,
		Weight::from_parts(50, 50),
		Weight::zero(),
	);
	assert_eq!(r, Outcome::Complete { used: Weight::from_parts(10, 10) });
	let uni_src = (ByGenesis([0; 32]), Teyrchain(42), Teyrchain(1)).into();
	assert_eq!(
		exported_xcm(),
		vec![(Pezkuwi, 403611790, uni_src, Here, expected_message, expected_hash)]
	);
}

#[test]
fn unpaid_execution_should_work() {
	// Bridge chain (assumed to be Relay) lets Teyrchain #1 have message execution for free.
	AllowUnpaidFrom::set(vec![[Teyrchain(1)].into()]);
	// Bridge chain (assumed to be Relay) lets Teyrchain #2 have message execution for free if it
	// asks.
	AllowExplicitUnpaidFrom::set(vec![[Teyrchain(2)].into()]);
	// Asking for unpaid execution of up to 9 weight on the assumption it is origin of #2.
	let message = Xcm(vec![UnpaidExecution {
		weight_limit: Limited(Weight::from_parts(9, 9)),
		check_origin: Some(Teyrchain(2).into()),
	}]);
	let mut hash = fake_message_hash(&message);
	let r = XcmExecutor::<TestConfig>::prepare_and_execute(
		Teyrchain(1),
		message.clone(),
		&mut hash,
		Weight::from_parts(50, 50),
		Weight::zero(),
	);
	assert_eq!(
		r,
		Outcome::Incomplete {
			used: Weight::from_parts(10, 10),
			error: InstructionError { index: 0, error: XcmError::BadOrigin },
		}
	);
	let r = XcmExecutor::<TestConfig>::prepare_and_execute(
		Teyrchain(2),
		message.clone(),
		&mut hash,
		Weight::from_parts(50, 50),
		Weight::zero(),
	);
	assert_eq!(
		r,
		Outcome::Incomplete {
			used: Weight::from_parts(10, 10),
			error: InstructionError { index: 0, error: XcmError::Barrier },
		}
	);

	let message = Xcm(vec![UnpaidExecution {
		weight_limit: Limited(Weight::from_parts(10, 10)),
		check_origin: Some(Teyrchain(2).into()),
	}]);
	let r = XcmExecutor::<TestConfig>::prepare_and_execute(
		Teyrchain(2),
		message.clone(),
		&mut hash,
		Weight::from_parts(50, 50),
		Weight::zero(),
	);
	assert_eq!(r, Outcome::Complete { used: Weight::from_parts(10, 10) });
}
