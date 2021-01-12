#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use frame_support::{
	decl_module, decl_storage, decl_event, decl_error, ensure, StorageValue, StorageDoubleMap, Parameter,
	traits::Randomness, RuntimeDebug, dispatch::{DispatchError, DispatchResult},
};
use sp_io::hashing::blake2_128;
use frame_system::ensure_signed;
use sp_runtime::traits::{AtLeast32BitUnsigned, Bounded, One, CheckedAdd};
use sp_std::prelude::*;

// #[cfg(test)]
// mod mock;

#[cfg(test)]
mod tests;


#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Kitty(pub [u8; 16]);


pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Randomness: Randomness<Self::Hash>;
	type KittyIndex: Parameter + AtLeast32BitUnsigned + Bounded + Default + Copy;
}

decl_storage! {
	trait Store for Module<T: Trait> as Kitties {
		/// Stores all the kitties, key is the kitty id
		pub Kitties get(fn kitties): double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) T::KittyIndex => Option<Kitty>;
		/// Stores the next kitty ID
        // pub NextKittyId get(fn next_kitty_id): T::KittyIndex;
        pub KittiesCount get(fn kitties_count): T::KittyIndex;
        pub KittyOwners get(fn kitty_owner): map hasher(blake2_128_concat) T::KittyIndex => Option<T::AccountId>;
        pub AccountKitties get(fn account_kitties): map hasher(blake2_128_concat) T::AccountId => Vec<(T::KittyIndex, Kitty)>;
	}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		<T as Trait>::KittyIndex,
	{
		/// A kitty is created. \[owner, kitty_id, kitty\]
        Created(AccountId, KittyIndex),
        Transfered(AccountId, AccountId, KittyIndex),
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

        #[weight = 0]
        pub fn create(origin) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let kitty_id = Self::next_kitty_id()?;
            
            let dna = Self::random_value(&sender);

            let kitty = Kitty(dna);

            Self::insert_kitty(&sender, kitty_id, kitty)?;

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