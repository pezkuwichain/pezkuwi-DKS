use crate as pezpallet_template;
use pezframe_support::derive_impl;
use pezsp_runtime::BuildStorage;

type Block = pezframe_system::mocking::MockBlock<Test>;

#[pezframe_support::runtime]
mod runtime {
	// The main runtime
	#[runtime::runtime]
	// Runtime Types to be generated
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Test;

	#[runtime::pezpallet_index(0)]
	pub type System = pezframe_system::Pezpallet<Test>;

	#[runtime::pezpallet_index(1)]
	pub type Template = pezpallet_template::Pezpallet<Test>;
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
}

impl pezpallet_template::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	pezframe_system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap()
		.into()
}
