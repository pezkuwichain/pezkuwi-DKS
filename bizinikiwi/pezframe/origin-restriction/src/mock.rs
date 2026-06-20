// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;
use codec::MaxEncodedLen;
use pezframe_support::{
	derive_impl,
	dispatch::{DispatchErrorWithPostInfo, GetDispatchInfo},
	pezpallet_prelude::TransactionValidityError,
	storage::with_transaction,
	traits::ContainsPair,
	weights::IdentityFee,
};
use pezpallet_transaction_payment::ConstFeeMultiplier;
use pezsp_core::{ConstU64, H256};
use pezsp_runtime::{
	testing::UintAuthorityId,
	traits::{Applyable, BlakeTwo256, Checkable, ConstUint, IdentityLookup},
	transaction_validity::{InvalidTransaction, TransactionSource},
	BuildStorage, DispatchError, FixedU128, TransactionOutcome,
};

pub type AccountId = <Test as pezframe_system::Config>::AccountId;
pub type BlockNumber = u64;

pub type TransactionExtension = (RestrictOrigin<Test>,);

pub type Header = pezsp_runtime::generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = pezsp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = pezsp_runtime::generic::UncheckedExtrinsic<
	AccountId,
	RuntimeCall,
	pezsp_runtime::testing::UintAuthorityId,
	TransactionExtension,
>;

pub const CALL_WEIGHT: u64 = 15;
pub const CALL_WEIGHT_EXCESS: u64 = 150;

/// A small mock pezpallet to test calls from within the runtime.
#[pezframe_support::pezpallet(dev_mode)]
pub mod mock_pallet {
	use super::{CALL_WEIGHT, CALL_WEIGHT_EXCESS};
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(Weight::from_parts(CALL_WEIGHT, 0))]
		pub fn do_something(_origin: OriginFor<T>) -> DispatchResult {
			Ok(())
		}

		#[pezpallet::call_index(1)]
		#[pezpallet::weight(Weight::from_parts(CALL_WEIGHT, 0))]
		pub fn do_something_refunded(_origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			Ok(Pays::No.into())
		}

		#[pezpallet::call_index(2)]
		#[pezpallet::weight(Weight::from_parts(CALL_WEIGHT_EXCESS, 0))]
		pub fn do_something_allowed_excess(_origin: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}
}

pezframe_support::construct_runtime!(
	pub enum Test {
		System: pezframe_system,
		MockPallet: mock_pallet,
		OriginsRestriction: crate,
		TransactionPayment: pezpallet_transaction_payment,
	}
);

/// Convenience aliases for the mock pezpallet calls.
pub type MockPalletCall = mock_pallet::Call<Test>;

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletInfo = PalletInfo;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type BlockHashCount = ConstU64<250>;
	type AccountData = ();
}

pub const RESTRICTED_ORIGIN_1: u64 = 1;
pub const RESTRICTED_ORIGIN_2: u64 = 2;
pub const NON_RESTRICTED_ORIGIN: u64 = 3;

#[derive(
	Encode,
	Decode,
	Clone,
	PartialEq,
	Eq,
	Debug,
	MaxEncodedLen,
	scale_info::TypeInfo,
	DecodeWithMemTracking,
)]
pub enum RuntimeRestrictedEntity {
	A,
	B,
}

impl RestrictedEntity<OriginCaller, u64> for RuntimeRestrictedEntity {
	fn allowance(&self) -> Allowance<u64> {
		Allowance { max: MAX_ALLOWANCE, recovery_per_block: ALLOWANCE_RECOVERY_PER_BLOCK }
	}

	fn restricted_entity(caller: &OriginCaller) -> Option<RuntimeRestrictedEntity> {
		match caller {
			OriginCaller::system(pezframe_system::Origin::<Test>::Signed(RESTRICTED_ORIGIN_1)) => {
				Some(RuntimeRestrictedEntity::A)
			},
			OriginCaller::system(pezframe_system::Origin::<Test>::Signed(RESTRICTED_ORIGIN_2)) => {
				Some(RuntimeRestrictedEntity::B)
			},
			_ => None,
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn benchmarked_restricted_origin() -> OriginCaller {
		OriginCaller::system(pezframe_system::Origin::<Test>::Signed(RESTRICTED_ORIGIN_1))
	}
}

pub struct TestOperationAllowedOneTimeExcess;
impl ContainsPair<RuntimeRestrictedEntity, RuntimeCall> for TestOperationAllowedOneTimeExcess {
	fn contains(entity: &RuntimeRestrictedEntity, call: &RuntimeCall) -> bool {
		matches!(
			(entity, call),
			(
				RuntimeRestrictedEntity::A,
				RuntimeCall::MockPallet(mock_pallet::Call::do_something_allowed_excess { .. })
			)
		)
	}
}

pub const MAX_ALLOWANCE: u64 = 100;
pub const ALLOWANCE_RECOVERY_PER_BLOCK: u64 = 5;

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type RestrictedEntity = RuntimeRestrictedEntity;
	type OperationAllowedOneTimeExcess = TestOperationAllowedOneTimeExcess;
}

pezframe_support::parameter_types! {
	pub ConstFeeMultiplierInner: FixedU128 = FixedU128::from_u32(1);
}

pub struct OnChargeTransaction;

impl pezpallet_transaction_payment::OnChargeTransaction<Test> for OnChargeTransaction {
	type Balance = u64;
	type LiquidityInfo = ();
	fn withdraw_fee(
		_who: &AccountId,
		_call: &RuntimeCall,
		_dispatch_info: &DispatchInfoOf<RuntimeCall>,
		_fee: Self::Balance,
		_tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		unimplemented!()
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn endow_account(_who: &AccountId, _amount: Self::Balance) {
		unimplemented!()
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn minimum_balance() -> Self::Balance {
		unimplemented!()
	}
	fn can_withdraw_fee(
		_who: &AccountId,
		_call: &RuntimeCall,
		_dispatch_info: &DispatchInfoOf<RuntimeCall>,
		_fee: Self::Balance,
		_tip: Self::Balance,
	) -> Result<(), TransactionValidityError> {
		unimplemented!()
	}
	fn correct_and_deposit_fee(
		_who: &AccountId,
		_dispatch_info: &DispatchInfoOf<RuntimeCall>,
		_post_info: &PostDispatchInfoOf<RuntimeCall>,
		_corrected_fee: Self::Balance,
		_tip: Self::Balance,
		_already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		unimplemented!()
	}
}

impl pezpallet_transaction_payment::TxCreditHold<Test> for OnChargeTransaction {
	type Credit = ();
}

impl pezpallet_transaction_payment::Config for Test {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type WeightToFee = IdentityFee<u64>;
	type LengthToFee = IdentityFee<u64>;
	type OperationalFeeMultiplier = ConstUint<1>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<ConstFeeMultiplierInner>;
	type OnChargeTransaction = OnChargeTransaction;
}

impl mock_pallet::Config for Test {}

/// Advance the chain to a certain block number.
#[allow(dead_code)]
pub fn advance_to(b: BlockNumber) {
	while System::block_number() < b {
		System::set_block_number(System::block_number() + 1);
	}
}

/// Advance the chain by a certain number of blocks.
pub fn advance_by(b: BlockNumber) {
	let initial_block = System::block_number();
	while System::block_number() < b + initial_block {
		System::set_block_number(System::block_number() + 1);
	}
}

/// Builds a new `TestExternalities`.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let storage = RuntimeGenesisConfig {
		system: Default::default(),
		transaction_payment: Default::default(),
	}
	.build_storage()
	.unwrap();
	pezsp_io::TestExternalities::from(storage)
}

/// We gather both error into a single type in order to do `assert_ok` and `assert_err` safely.
/// Otherwise, we can easily miss the inner error in a `Resut<Resut<_, _>, _>`.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TransactionExecutionError {
	Validity(TransactionValidityError),
	// This ignores the post info.
	Dispatch(DispatchErrorWithPostInfo),
}

impl From<DispatchErrorWithPostInfo> for TransactionExecutionError {
	fn from(e: DispatchErrorWithPostInfo) -> Self {
		Self::Dispatch(e)
	}
}

impl From<TransactionValidityError> for TransactionExecutionError {
	fn from(e: TransactionValidityError) -> Self {
		Self::Validity(e)
	}
}

impl From<DispatchError> for TransactionExecutionError {
	fn from(e: DispatchError) -> Self {
		Self::Dispatch(e.into())
	}
}

impl From<InvalidTransaction> for TransactionExecutionError {
	fn from(e: InvalidTransaction) -> Self {
		Self::Validity(e.into())
	}
}

/// Execute a transaction with the given origin, call and transaction extension.
pub fn exec_signed_tx(
	who: u64,
	call: impl Into<RuntimeCall>,
) -> Result<(), TransactionExecutionError> {
	let tx_ext = (RestrictOrigin::<Test>::new(true),);
	let tx = UncheckedExtrinsic::new_signed(call.into(), who, UintAuthorityId(who), tx_ext);

	exec_tx(tx)
}

/// Execute a transaction with the given origin, call and transaction extension. but with the
/// `RestrictOrigin` disabled.
pub fn exec_signed_tx_disabled(
	who: u64,
	call: impl Into<RuntimeCall>,
) -> Result<(), TransactionExecutionError> {
	// Construct the extension with `false` for the enabling boolean.
	let tx_ext = (RestrictOrigin::<Test>(false, Default::default()),);
	let tx = UncheckedExtrinsic::new_signed(call.into(), who, UintAuthorityId(who), tx_ext);

	exec_tx(tx)
}

/// Execute a transaction with the given origin, call and transaction extension.
pub fn exec_tx(tx: UncheckedExtrinsic) -> Result<(), TransactionExecutionError> {
	let info = tx.get_dispatch_info();
	let len = tx.encoded_size();

	let checked = Checkable::check(tx, &pezframe_system::ChainContext::<Test>::default())?;

	with_transaction(|| {
		let validity = checked.validate::<Test>(TransactionSource::External, &info, len);
		TransactionOutcome::Rollback(Result::<_, DispatchError>::Ok(validity))
	})??;

	checked.apply::<Test>(&info, len)??;

	Ok(())
}
