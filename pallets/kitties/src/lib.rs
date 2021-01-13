#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use frame_support::{
    Parameter, RuntimeDebug, StorageDoubleMap, StorageValue, 
    decl_error, decl_event, decl_module, decl_storage, 
    dispatch::{ DispatchError, DispatchResult }, ensure, 
    traits::{ Currency, ReservableCurrency, Randomness },
};
use sp_io::hashing::{blake2_128, twox_64};
use frame_system::{self as system, ensure_signed};
use sp_runtime::traits::{AtLeast32BitUnsigned, Bounded, One, CheckedAdd};
use sp_std::prelude::*;

// #[cfg(test)]
// mod mock;

#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Kitty(pub [u8; 16]);

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct LockId(pub [u8; 8]);


type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Randomness: Randomness<Self::Hash>;
    type KittyIndex: Parameter + AtLeast32BitUnsigned + Bounded + Default + Copy;
    // type Currency: LockableCurrency<Self::AccountId, Moment=Self::BlockNumber>;

    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Kitties {
		/// Stores all the kitties, key is the kitty id
        pub Kitties get(fn kitties): double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) T::KittyIndex => Option<Kitty>;
        // Kitty 总数
        pub KittiesCount get(fn kitties_count): T::KittyIndex;
        // Kitty拥有者
        pub KittyOwners get(fn kitty_owner): map hasher(blake2_128_concat) T::KittyIndex => Option<T::AccountId>;
        // 某账户所有的Kitty
        pub AccountKitties get(fn account_kitties): map hasher(blake2_128_concat) T::AccountId => Vec<(T::KittyIndex, Kitty)>;
        pub KittyLockId get(fn lock_id): map hasher(blake2_128_concat) T::KittyIndex => Option<LockId>;
	}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
        <T as Trait>::KittyIndex,
        Balance = BalanceOf<T>,
        BlockNumber = <T as system::Trait>::BlockNumber,
	{
		/// A kitty is created. \[owner, kitty_id, kitty\]
        Created(AccountId, KittyIndex),
        Transfered(AccountId, AccountId, KittyIndex),

        LockFunds(AccountId, Balance, BlockNumber),
		UnlockFunds(AccountId, Balance, BlockNumber),
		// sender, dest, amount, block number
		TransferFunds(AccountId, AccountId, Balance, BlockNumber),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		KittiesIdOverflow,
		InvalidKittyId,
        SameGender,

        KittiesCountOverflow,
        InvalidaKittyId,
        RequireDifferentParent,
        AccountNotExist,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 10_000]
		pub fn reserve_funds(origin, amount: BalanceOf<T>) -> DispatchResult {
			let locker = ensure_signed(origin)?;

			T::Currency::reserve(&locker, amount)
					.map_err(|_| "locker can't afford to lock the amount requested")?;

			let now = <system::Module<T>>::block_number();

			Self::deposit_event(RawEvent::LockFunds(locker, amount, now));
			Ok(())
		}

		/// Unreserves the specified amount of funds from the caller
		#[weight = 10_000]
		pub fn unreserve_funds(origin, amount: BalanceOf<T>) -> DispatchResult {
			let unlocker = ensure_signed(origin)?;

			T::Currency::unreserve(&unlocker, amount);
			// ReservableCurrency::unreserve does not fail (it will lock up as much as amount)

			let now = <system::Module<T>>::block_number();

			Self::deposit_event(RawEvent::UnlockFunds(unlocker, amount, now));
			Ok(())
		}

        #[weight = 1000]
        pub fn create(origin, amount: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin.clone())?;

            let kitty_id = Self::next_kitty_id()?;

            let dna = Self::random_value(&sender);

            let kitty = Kitty(dna);

            Self::insert_kitty(&sender, kitty_id, kitty)?;

            Self::reserve_funds(origin, amount)?;

            Self::deposit_event(RawEvent::Created(sender, kitty_id));

            Ok(())
        }

        #[weight = 0]
        pub fn transfer(origin, to: T::AccountId, kitty_id: T::KittyIndex) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let kitty = Kitties::<T>::take(&sender, kitty_id).ok_or(Error::<T>::InvalidaKittyId)?;

            // update accountKitties
            let sender_kitty_vec = AccountKitties::<T>::take(&sender);
            let mut to_kitty_vec = AccountKitties::<T>::take(&to);
            let mut new_sender_k_vec = Vec::new();
            for (kid, kt) in sender_kitty_vec.iter() {
                if kid != &kitty_id {
                    new_sender_k_vec.push((*kid, kt));
                } else {
                    to_kitty_vec.push((*kid, kitty.clone()));
                }
            }
            AccountKitties::<T>::insert(&sender, new_sender_k_vec);
            AccountKitties::<T>::insert(&to, to_kitty_vec);
            KittyOwners::<T>::insert(kitty_id, to.clone());

            Self::deposit_event(RawEvent::Transfered(sender, to, kitty_id));
            Ok(())
        }

        #[weight = 0]
        pub fn breed(origin, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) {
            let sender = ensure_signed(origin)?;

            let new_kitty_id = Self::do_breed(&sender, kitty_id_1, kitty_id_2)?;

            Self::deposit_event(RawEvent::Created(sender, new_kitty_id));
        }
	}
}


fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
    (selector & dna1) | (!selector & dna2)
}

impl<T: Trait> Module<T> {
    fn next_kitty_id() -> sp_std::result::Result<T::KittyIndex, DispatchError> {
        // let kitty_id = Self::kitties_count();
        let kitty_id = Self::kitties_count().checked_add(&One::one()).ok_or(Error::<T>::KittiesCountOverflow)?;
        Ok(kitty_id)
    }

    fn random_value(sender: &T::AccountId) -> [u8;16] {
        let payload = (
            T::Randomness::random_seed(),
            &sender,
            <frame_system::Module<T>>::extrinsic_index(),
        );

        payload.using_encoded(blake2_128)
    }

    fn random_lock_id(sender: &T::AccountId) -> [u8; 8] {
        let payload = (
            T::Randomness::random_seed(),
            &sender,
            <frame_system::Module<T>>::extrinsic_index(),
        );

        payload.using_encoded(twox_64)
    }

    fn insert_kitty(owner: &T::AccountId, kitty_id: T::KittyIndex, kitty: Kitty) -> DispatchResult {
        Kitties::<T>::insert(&owner, kitty_id, kitty.clone());
        KittyOwners::<T>::insert(kitty_id, &owner);

        let mut kitty_vec = AccountKitties::<T>::take(&owner);
        kitty_vec.push((kitty_id, kitty));
        AccountKitties::<T>::insert(&owner, kitty_vec);
        KittiesCount::<T>::put(kitty_id);
        Ok(())
    }

    fn do_breed(sender: &T::AccountId, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) -> sp_std::result::Result<T::KittyIndex, DispatchError> {
        let kitty1 = Self::kitties(&sender, kitty_id_1).ok_or(Error::<T>::InvalidaKittyId)?;
        let kitty2 = Self::kitties(&sender, kitty_id_2).ok_or(Error::<T>::InvalidaKittyId)?;

        ensure!(kitty_id_1 != kitty_id_2, Error::<T>::RequireDifferentParent);
        let kitty_id = Self::next_kitty_id()?;

        let kitty1_dna = kitty1.0;
        let kitty2_dna = kitty2.0;
        let selector = Self::random_value(&sender);
        let mut new_dna = [0u8; 16];
        for i in 0..kitty1_dna.len() {
            new_dna[i] = combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
        }
        Self::insert_kitty(sender, kitty_id, Kitty(new_dna))?;
        Ok(kitty_id)
    }
}