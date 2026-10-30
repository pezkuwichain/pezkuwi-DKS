// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarks for pezpallet-navnisan. Every one sets up the worst case its weight is charged
//! for: `d` ancestors to walk, `t` text records to clear.

use crate::*;
use alloc::{vec, vec::Vec};
use pezframe_benchmarking::v2::*;
use pezframe_support::traits::{fungible::Mutate, EnsureOrigin, Get};
use pezframe_system::RawOrigin;
use pezsp_runtime::traits::Bounded;

const SEED: u32 = 0;
const START: Moment = 1_000 * MILLIS_PER_DAY;

fn funded<T: Config>(name: &'static str, i: u32) -> T::AccountId {
	let who: T::AccountId = account(name, i, SEED);
	let _ = T::Currency::set_balance(&who, BalanceOf::<T>::max_value() / 4u32.into());
	who
}

fn setup<T: Config>() {
	T::BenchmarkHelper::set_time(START);
	if Prices::<T>::get().len5plus.is_zero() {
		let one: BalanceOf<T> = 1_000u32.into();
		Prices::<T>::put(PriceBands {
			len3: one * 50u32.into(),
			len4: one * 10u32.into(),
			len5plus: one,
		});
	}
}

fn cid(byte: u8) -> Cid {
	let mut v = vec![0xe3, 0x01, 0x01, 0x70, 0x12, 0x20];
	v.extend([byte; 32]);
	v.try_into().unwrap()
}

fn label_of(i: u32) -> Vec<u8> {
	// Five letters and up, never two hyphens: "nameaa", "nameab", ...
	let mut l = b"name".to_vec();
	l.push(b'a' + (i / 26 % 26) as u8);
	l.push(b'a' + (i % 26) as u8);
	l
}

/// Buy a top-level name for `who` through the real path.
fn buy<T: Config>(who: &T::AccountId, label: &[u8]) -> NameHash {
	let secret = [7u8; 32];
	let now = Pezpallet::<T>::now();
	Commitments::<T>::insert(
		commitment(label, who, who, &secret),
		(who.clone(), now.saturating_sub(T::MinCommitmentAge::get()), BalanceOf::<T>::zero()),
	);
	Pezpallet::<T>::register(
		RawOrigin::Signed(who.clone()).into(),
		label.to_vec(),
		who.clone(),
		secret,
		1,
	)
	.expect("benchmark setup: register");
	child_node(&kurd_node(), label)
}

/// A name `depth` labels deep (1 = top level), every level managed by `who`.
fn chain<T: Config>(who: &T::AccountId, depth: u32) -> NameHash {
	let mut node = buy::<T>(who, b"chainroot");
	for i in 1..depth {
		Pezpallet::<T>::set_subname(
			RawOrigin::Signed(who.clone()).into(),
			node,
			label_of(i),
			who.clone(),
			None,
		)
		.expect("benchmark setup: subname");
		node = child_node(&node, &label_of(i));
	}
	node
}

fn fill_texts<T: Config>(who: &T::AccountId, node: NameHash, t: u32) {
	for i in 0..t {
		let key: label::TextKey = label_of(i).try_into().unwrap();
		let value: label::TextValue =
			vec![b'v'; label::MAX_TEXT_VALUE as usize].try_into().unwrap();
		Pezpallet::<T>::set_text(RawOrigin::Signed(who.clone()).into(), node, key, Some(value))
			.expect("benchmark setup: text");
	}
}

/// A granted top-level name managed by `who`: institutional, so changes queue.
fn institution<T: Config>(who: &T::AccountId) -> NameHash {
	Pezpallet::<T>::do_grant(None, b"kurum", Owner::Account(who.clone())).expect("grant");
	child_node(&kurd_node(), b"kurum")
}

fn grant_origin<T: Config>() -> T::RuntimeOrigin {
	T::GrantOrigin::try_successful_origin().expect("GrantOrigin has a successful origin")
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn commit() {
		setup::<T>();
		let who = funded::<T>("who", 0);
		#[extrinsic_call]
		_(RawOrigin::Signed(who), H256::repeat_byte(1));
	}

	#[benchmark]
	fn register(t: Linear<0, { label::MAX_TEXTS }>) {
		setup::<T>();
		let old = funded::<T>("old", 0);
		let node = buy::<T>(&old, b"benchname");
		fill_texts::<T>(&old, node, t);
		let later = START + YEAR + T::GracePeriod::get() + 1;
		T::BenchmarkHelper::set_time(later);
		let who = funded::<T>("who", 0);
		let secret = [9u8; 32];
		Commitments::<T>::insert(
			commitment(b"benchname", &who, &who, &secret),
			(who.clone(), later - T::MinCommitmentAge::get(), BalanceOf::<T>::zero()),
		);
		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()), b"benchname".to_vec(), who.clone(), secret, 1);
		assert_eq!(Names::<T>::get(node).unwrap().owner, Owner::Account(who));
	}

	#[benchmark]
	fn renew() {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = buy::<T>(&who, b"benchname");
		#[extrinsic_call]
		_(RawOrigin::Signed(who), node, 1);
	}

	#[benchmark]
	fn transfer(d: Linear<1, { label::MAX_DEPTH }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = chain::<T>(&who, d);
		let to: T::AccountId = account("to", 0, SEED);
		#[extrinsic_call]
		_(RawOrigin::Signed(who), node, to);
	}

	#[benchmark]
	fn set_account(d: Linear<1, { label::MAX_DEPTH }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = chain::<T>(&who, d);
		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()), node, Some(who.clone()));
	}

	/// The institutional path, which also writes the pending change.
	#[benchmark]
	fn set_content(d: Linear<1, { label::MAX_DEPTH }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let mut node = institution::<T>(&who);
		for i in 1..d {
			Pezpallet::<T>::do_grant(Some(node), &label_of(i), Owner::Account(who.clone()))
				.unwrap();
			node = child_node(&node, &label_of(i));
		}
		#[extrinsic_call]
		_(RawOrigin::Signed(who), node, Some(Content::Ipfs(cid(1))));
		assert!(PendingChanges::<T>::get(node, 1).is_some());
	}

	#[benchmark]
	fn anchor(d: Linear<1, { label::MAX_DEPTH }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = chain::<T>(&who, d);
		let feed =
			Content::Feed { publisher: [1; 32], anchorer: who.clone(), anchor: cid(1), seq: 1 };
		Names::<T>::mutate(node, |r| r.as_mut().unwrap().content = Some(feed));
		#[extrinsic_call]
		_(RawOrigin::Signed(who), node, cid(2), 2);
	}

	#[benchmark]
	fn set_text(d: Linear<1, { label::MAX_DEPTH }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = chain::<T>(&who, d);
		// Replacing an existing record: release, then hold again.
		fill_texts::<T>(&who, node, 1);
		let key: label::TextKey = label_of(0).try_into().unwrap();
		let value: label::TextValue =
			vec![b'w'; label::MAX_TEXT_VALUE as usize].try_into().unwrap();
		#[extrinsic_call]
		_(RawOrigin::Signed(who), node, key, Some(value));
	}

	/// Replaces an existing subname that holds `t` text records.
	#[benchmark]
	fn set_subname(d: Linear<1, { label::MAX_DEPTH - 1 }>, t: Linear<0, { label::MAX_TEXTS }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let parent = chain::<T>(&who, d);
		Pezpallet::<T>::set_subname(
			RawOrigin::Signed(who.clone()).into(),
			parent,
			b"x".to_vec(),
			who.clone(),
			None,
		)
		.unwrap();
		fill_texts::<T>(&who, child_node(&parent, b"x"), t);
		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()), parent, b"x".to_vec(), who.clone(), None);
	}

	#[benchmark]
	fn remove_subname(t: Linear<0, { label::MAX_TEXTS }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = chain::<T>(&who, 2);
		fill_texts::<T>(&who, node, t);
		#[extrinsic_call]
		_(RawOrigin::Signed(who), node);
	}

	#[benchmark]
	fn set_primary(d: Linear<1, { label::MAX_DEPTH }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = chain::<T>(&who, d);
		#[extrinsic_call]
		_(RawOrigin::Signed(who), node);
	}

	#[benchmark]
	fn clear_primary() {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = buy::<T>(&who, b"benchname");
		Primary::<T>::insert(&who, node);
		#[extrinsic_call]
		_(RawOrigin::Signed(who));
	}

	#[benchmark]
	fn grant(d: Linear<1, { label::MAX_DEPTH }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let origin = grant_origin::<T>();
		let parent = if d == 1 {
			None
		} else {
			let mut node = institution::<T>(&who);
			for i in 2..d {
				Pezpallet::<T>::do_grant(Some(node), &label_of(i), Owner::Account(who.clone()))
					.unwrap();
				node = child_node(&node, &label_of(i));
			}
			Some(node)
		};
		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, parent, b"granted".to_vec(), Owner::Account(who));
	}

	#[benchmark]
	fn revoke(t: Linear<0, { label::MAX_TEXTS }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = institution::<T>(&who);
		fill_texts::<T>(&who, node, t);
		let origin = grant_origin::<T>();
		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, node);
	}

	#[benchmark]
	fn force_change() {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = institution::<T>(&who);
		let origin = grant_origin::<T>();
		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, node, Change::Content(Some(Content::Ipfs(cid(1)))));
	}

	#[benchmark]
	fn cancel_change() {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = institution::<T>(&who);
		Pezpallet::<T>::set_content(
			RawOrigin::Signed(who).into(),
			node,
			Some(Content::Ipfs(cid(1))),
		)
		.unwrap();
		let origin = grant_origin::<T>();
		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, node, 1);
	}

	#[benchmark]
	fn enact_change() {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = institution::<T>(&who);
		Pezpallet::<T>::set_content(
			RawOrigin::Signed(who.clone()).into(),
			node,
			Some(Content::Ipfs(cid(1))),
		)
		.unwrap();
		T::BenchmarkHelper::set_time(START + T::InstitutionDelay::get());
		#[extrinsic_call]
		_(RawOrigin::Signed(who), node, 1);
	}

	#[benchmark]
	fn schedule_revocation() {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = buy::<T>(&who, b"benchname");
		let origin = T::DisputeOrigin::try_successful_origin().expect("DisputeOrigin");
		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, node, H256::repeat_byte(3));
	}

	#[benchmark]
	fn cancel_revocation() {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = buy::<T>(&who, b"benchname");
		PendingRevocations::<T>::insert(
			node,
			Revocation { due: START, reg: 1, reason: H256::zero() },
		);
		let origin = T::DisputeOrigin::try_successful_origin().expect("DisputeOrigin");
		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, node);
	}

	#[benchmark]
	fn enact_revocation(t: Linear<0, { label::MAX_TEXTS }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = buy::<T>(&who, b"benchname");
		fill_texts::<T>(&who, node, t);
		let reg = Names::<T>::get(node).unwrap().reg;
		PendingRevocations::<T>::insert(node, Revocation { due: START, reg, reason: H256::zero() });
		#[extrinsic_call]
		_(RawOrigin::Signed(who), node);
		assert!(Names::<T>::get(node).is_none());
	}

	#[benchmark]
	fn lock() {
		setup::<T>();
		let origin = grant_origin::<T>();
		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, b"kobani".to_vec(), 5);
	}

	#[benchmark]
	fn unlock() {
		setup::<T>();
		let node = child_node(&kurd_node(), b"kobani");
		Locked::<T>::insert(node, 5);
		let origin = grant_origin::<T>();
		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, node);
	}

	#[benchmark]
	fn set_prices() {
		setup::<T>();
		let origin = T::PriceOrigin::try_successful_origin().expect("PriceOrigin");
		let p = Prices::<T>::get();
		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, p);
	}

	/// A subname `d` deep whose top-level lease has lapsed, holding `t` text records.
	#[benchmark]
	fn reap(d: Linear<1, { label::MAX_DEPTH }>, t: Linear<0, { label::MAX_TEXTS }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = chain::<T>(&who, d);
		fill_texts::<T>(&who, node, t);
		T::BenchmarkHelper::set_time(START + YEAR + T::GracePeriod::get() + 1);
		#[extrinsic_call]
		_(RawOrigin::Signed(who), node);
		assert!(Names::<T>::get(node).is_none());
	}

	#[benchmark]
	fn reap_primary(d: Linear<1, { label::MAX_DEPTH }>) {
		setup::<T>();
		let who = funded::<T>("who", 0);
		let node = chain::<T>(&who, d);
		Primary::<T>::insert(&who, node);
		Names::<T>::mutate(node, |r| r.as_mut().unwrap().account = None);
		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()), who.clone());
	}

	#[benchmark]
	fn remove_commitment() {
		setup::<T>();
		let who = funded::<T>("who", 0);
		Pezpallet::<T>::commit(RawOrigin::Signed(who.clone()).into(), H256::repeat_byte(1))
			.unwrap();
		T::BenchmarkHelper::set_time(START + T::MaxCommitmentAge::get() + 1);
		#[extrinsic_call]
		_(RawOrigin::Signed(who), H256::repeat_byte(1));
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
