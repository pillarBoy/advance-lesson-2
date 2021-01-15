pub use super::*;

pub use std::cell::RefCell;
use sp_core::H256;
pub use frame_support::{
    impl_outer_origin, impl_outer_event, parameter_types, weights::Weight,
	assert_ok, assert_noop,
	traits::{Currency, Get,},
};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
};

use pallet_balances as balances;

use frame_system as system;

impl_outer_origin! {
	pub enum Origin for Test {}
}

pub(crate) type Balance = u128;


pub mod kitties {
	// Re-export needed for `impl_outer_event!`.
	pub use super::super::*;
}

pub struct ExistentialDeposit;
impl Get<Balance> for ExistentialDeposit {
	fn get() -> Balance {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
	}
}

impl_outer_event! {
	pub enum Event for Test {
		frame_system<T>,
		kitties<T>,
		balances<T>,
	}
}
// Configure a mock runtime to test the pallet.

pub type KModule = Module<Test>;
pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	// pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	pub const AvailableBlockRatio: Perbill = Perbill::one();

	// pub const ExistentialDeposit: u64 = 1;
	pub const TransferFee: u64 = 0;
	pub const CreationFee: u64 = 0;

	pub const KittyReserveFundsConst: u64 = 10_000_000_000_000;
}

impl system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

impl pallet_balances::Trait for Test {
	type MaxLocks = ();
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

thread_local! {
	static RANDOM_PAYLOAD: RefCell<H256> = RefCell::new(Default::default());
	static EXISTENTIAL_DEPOSIT: RefCell<Balance> = RefCell::new(0);
}

pub struct MockRandom;

impl Randomness<H256> for MockRandom {
    fn random(_subject: &[u8]) -> H256 {
        RANDOM_PAYLOAD.with(|v| *v.borrow())
    }
}

impl Trait for Test {
	type Event = Event;
	type Randomness = MockRandom;
	type KittyIndex = u32;
	type Currency = Balances;
	type KittyReserveFunds = u8;
}


// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	balances::GenesisConfig::<Test> {
		// Provide some initial balances
		balances: vec![(1, 10000), (2, 11000), (3, 12000), (4, 13000), (5, 14000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	let mut ext: sp_io::TestExternalities = t.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn last_event() -> Event {
    System::events().last().unwrap().event.clone()
}