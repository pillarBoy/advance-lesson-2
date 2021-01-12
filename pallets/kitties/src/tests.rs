use super::*;

use std::cell::RefCell;
use sp_core::H256;
use frame_support::{
    impl_outer_origin, impl_outer_event, parameter_types, weights::Weight,
	assert_ok, assert_noop,
	traits::{Currency, Get,},
};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
};
use crate::*;
use pallet_balances as balances;


use frame_system as system;
use pallet_session as session;



impl_outer_origin! {
	pub enum Origin for Test {}
}

pub(crate) type Balance = u128;


mod kitties {
	// Re-export needed for `impl_outer_event!`.
	pub use super::super::*;
}

// impl_outer_event! {
// 	pub enum MetaEvent for Test {
// 		system<T>,
// 		balances<T>,
// 		session,
// 		kitties<T>,
// 	}
// }

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
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
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

}



// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();
	t.execute_with(|| System::set_block_number(1) );
    t
}

pub fn last_event() -> Event {
    System::events().last().unwrap().event.clone()
}


#[test]
fn can_create_kitty() {
	new_test_ext().execute_with(|| {
		assert_ok!(KModule::create(Origin::signed(1)));
		
        let kt = Kitty([39, 140, 77, 194, 163, 1, 154, 220, 108, 18, 30, 32, 100, 223, 46, 1]);
        assert_eq!(KModule::kitties(1, 1), Some(kt.clone()));
		assert_eq!(KModule::kitties_count(), 1);
		
        assert_eq!(last_event(), Event::kitties(RawEvent::Created(1, 1)));
	});
}

#[test]
fn can_transfer() {
    new_test_ext().execute_with(|| {
        assert_ok!(KModule::create(Origin::signed(1)));

        assert_eq!(KModule::kitties_count(), 1);

		// kitty id 不正确  不可以转移
        assert_noop!(KModule::transfer(Origin::signed(1), 2, 0), Error::<Test>::InvalidaKittyId);

        assert_ok!(KModule::transfer(Origin::signed(1), 2, 1));

        assert_eq!(last_event(), Event::kitties(RawEvent::Transfered(1, 2, 1)));
    });
}

#[test]
fn can_breed() {
	new_test_ext().execute_with(|| {
		assert_ok!(KModule::create(Origin::signed(1)));
		assert_ok!(KModule::create(Origin::signed(1)));

		assert_noop!(KModule::breed(Origin::signed(1), 0, 3), Error::<Test>::InvalidaKittyId);

		assert_noop!(KModule::breed(Origin::signed(1), 1, 1), Error::<Test>::RequireDifferentParent);

		assert_ok!(KModule::breed(Origin::signed(1), 1, 2));

		let kt = Kitty([39, 140, 77, 194, 163, 1, 154, 220, 108, 18, 30, 32, 100, 223, 46, 1]);
		assert_eq!(KModule::kitties(1, 3), Some(kt.clone()));
		assert_eq!(KModule::kitties_count(), 3);

		assert_eq!(last_event(), Event::kitties(RawEvent::Created(1, 3)));
	})
}