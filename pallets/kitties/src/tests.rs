use crate::{Error, mock::*};
use frame_support::{assert_ok, assert_noop};


#[test]
fn can_create_kitty() {
	new_test_ext().execute_with(|| {
		assert_ok!(KModule::create(Origin::signed(1), 1230));
		
        let kt = Kitty([39, 140, 77, 194, 163, 1, 154, 220, 108, 18, 30, 32, 100, 223, 46, 1]);
        assert_eq!(KModule::kitties(1, 1), Some(kt.clone()));
		assert_eq!(KModule::kitties_count(), 1);
		
        assert_eq!(last_event(), Event::kitties(RawEvent::Created(1, 1)));
	});
}

#[test]
fn can_reserve_funds() {
	new_test_ext().execute_with(|| {
		assert_ok!(KModule::reserve_funds(Origin::signed(1), 1, 100));

		assert_eq!(last_event(), Event::kitties(RawEvent::LockFunds(1, 100, 1)));

		// Test and see if (1, 5000) holds 账户可转账余额
		assert_eq!(Balances::free_balance(&1), 9900);
		// 账户锁仓余额
		assert_eq!(Balances::reserved_balance(&1), 100);
	});
}

#[test]
fn can_unreserve_and_transfer() {
	new_test_ext().execute_with(|| {
		assert_ok!(KModule::reserve_funds(Origin::signed(1), 1, 100));
		// Test and see if (1, 5000) holds 账户可转账余额
		assert_eq!(Balances::free_balance(&1), 9900);
		// 账户锁仓余额
		assert_eq!(Balances::reserved_balance(&1), 100);
		assert_eq!(last_event(), Event::kitties(RawEvent::LockFunds(1, 100, 1)));

		// 转移质押token
		assert_ok!(KModule::unreserve_and_transfer(Origin::signed(1), 1, 2, 100));
		// 转移质押event
		assert_eq!(last_event(), Event::kitties(RawEvent::TransferFunds(1, 2, 100, 1)));

		assert_eq!(Balances::reserved_balance(&1), 0);
		assert_eq!(Balances::reserved_balance(&2), 0);
		assert_eq!(Balances::free_balance(&1), 9900);
		assert_eq!(Balances::free_balance(&2), 11100);

	});
}

#[test]
fn can_transfer() {
    new_test_ext().execute_with(|| {
        assert_ok!(KModule::create(Origin::signed(1), 1230));

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
		assert_ok!(KModule::create(Origin::signed(1), 1230));
		assert_ok!(KModule::create(Origin::signed(1), 1230));

		assert_noop!(KModule::breed(Origin::signed(1), 0, 3), Error::<Test>::InvalidaKittyId);

		assert_noop!(KModule::breed(Origin::signed(1), 1, 1), Error::<Test>::RequireDifferentParent);

		assert_ok!(KModule::breed(Origin::signed(1), 1, 2));

		let kt = Kitty([39, 140, 77, 194, 163, 1, 154, 220, 108, 18, 30, 32, 100, 223, 46, 1]);
		assert_eq!(KModule::kitties(1, 3), Some(kt.clone()));
		assert_eq!(KModule::kitties_count(), 3);

		assert_eq!(last_event(), Event::kitties(RawEvent::Created(1, 3)));
	})
}