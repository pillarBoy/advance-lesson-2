#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use frame_support::{
    Parameter, RuntimeDebug, StorageDoubleMap, StorageValue, 
    decl_error, decl_event, decl_module, decl_storage,  
    dispatch::{ DispatchError, DispatchResult }, ensure, 
    traits::Get,
    traits::{ Currency, ExistenceRequirement::AllowDeath, ReservableCurrency, Randomness },
};
use sp_io::hashing::{blake2_128};
use frame_system::{self as system, ensure_signed};
use sp_runtime::traits::{AtLeast32BitUnsigned, Bounded, One, CheckedAdd};
use sp_std::prelude::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Kitty(pub [u8; 16]);

// 感觉这个结构存储的数据有点多，不是很高效
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct KittyNode<T: Trait> {
    // 自己
    _self: T::KittyIndex,
    // 父母双方，create的kitty可能是None
    companion: Option<(T::KittyIndex, T::KittyIndex)>,
    // kitty的children
    children: Vec<T::KittyIndex>,
}

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Randomness: Randomness<Self::Hash>;
    type KittyIndex: Parameter + AtLeast32BitUnsigned + Bounded + Default + Copy;
    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
    type KittyReserveFunds: Get<BalanceOf<Self>>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Kitties {
		// kitty账户 kitty id映射
        pub Kitties get(fn kitties): double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) T::KittyIndex => Option<Kitty>;
        // Kitty 总数
        pub KittiesCount get(fn kitties_count): T::KittyIndex;
        // Kitty拥有者
        pub KittyOwners get(fn kitty_owner): map hasher(blake2_128_concat) T::KittyIndex => Option<T::AccountId>;
        // 某账户所有的Kitty
        pub AccountKitties get(fn account_kitties): map hasher(blake2_128_concat) T::AccountId => Vec<(T::KittyIndex, Kitty)>;
        // kitty 对应的质押数量
        pub KittyLockAmount get(fn lock_amount): map hasher(blake2_128_concat) T::KittyIndex => Option<BalanceOf<T>>;
        // kitty 对应关系
        pub KittyNodeStorage get(fn get_kitty_from_node): Vec<KittyNode<T>>;
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
        KittiesCountOverflow,
        InvalidaKittyId,
        RequireDifferentParent,
        AccountNotExist,

        BalanceNotEnough,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 0]
		pub fn reserve_funds(origin, locker: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let _sender = ensure_signed(origin)?;

            // 这里其实已经判断了余额不足的 但是这个Error Event 还没找到怎么发送 BalanceNotEnough
            T::Currency::reserve(&locker, amount)
                .map_err(|_| Error::<T>::BalanceNotEnough)?;

            let now = <system::Module<T>>::block_number();
            Self::deposit_event(RawEvent::LockFunds(locker, amount, now));
			
			Ok(())
		}
        
        #[weight = 10_000]
		pub fn unreserve_and_transfer(
			origin,
			to_punish: T::AccountId,
			dest: T::AccountId,
			collateral: BalanceOf<T>
		) -> DispatchResult {
			let _ = ensure_signed(origin)?; // dangerous because can be called with any signature (so dont do this in practice ever!)

						// If collateral is bigger than to_punish's reserved_balance, store what's left in overdraft.
			let overdraft = T::Currency::unreserve(&to_punish, collateral);

			T::Currency::transfer(&to_punish, &dest, collateral - overdraft, AllowDeath)?;

			let now = <system::Module<T>>::block_number();
			Self::deposit_event(RawEvent::TransferFunds(to_punish, dest, collateral - overdraft, now));

			Ok(())
		}

        #[weight = 1000]
        pub fn create(origin) -> DispatchResult {
            let sender = ensure_signed(origin.clone())?;

            let kitty_id = Self::next_kitty_id()?;

            let dna = Self::random_value(&sender);

            let kitty = Kitty(dna);

            Self::insert_kitty(&sender, kitty_id, kitty)?;

            let amount = T::KittyReserveFunds::get();

            KittyLockAmount::<T>::insert(kitty_id, amount);

            Self::reserve_funds(origin, sender.clone(), amount)?;

            Self::deposit_event(RawEvent::Created(sender.clone(), kitty_id));

            // 更新kitty node
            let mut node_vec = KittyNodeStorage::<T>::take();
            let node = KittyNode {
                _self: kitty_id,
                children: Vec::new(),
                companion: None, // 
            };
            node_vec.push(node);
            // 更新kittynode 关系vec
            KittyNodeStorage::<T>::put(node_vec);

            Ok(())
        }

        #[weight = 0]
        pub fn transfer(origin, to: T::AccountId, kitty_id: T::KittyIndex) -> DispatchResult {
            let sender = ensure_signed(origin.clone())?;
            let kitty = Kitties::<T>::take(&sender, kitty_id).ok_or(Error::<T>::InvalidaKittyId)?;

            // 查找到需要转移到那只 kitty 变更 kitty的所有者关系
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
            KittyOwners::<T>::insert(&kitty_id, to.clone());

            // 获取kitty的质押数量
            let amount = Self::lock_amount(kitty_id).ok_or(Error::<T>::InvalidaKittyId)?;
            // 解除质押，并转移质押到拥有者账号
            Self::unreserve_and_transfer(origin.clone(), sender.clone(), to.clone(), amount)?;
            // 把质押的token 质押到拥有者账号里 (会不会产生在上面解除质押，转移的过程中，toke还没到账，然后账户上没有足够的token去质押的情况呢？也就是，这里是同步的，不是异步执行的吧)
            Self::reserve_funds(origin, to.clone(), amount)?;

            Self::deposit_event(RawEvent::Transfered(sender, to, kitty_id));
            Ok(())
        }

        #[weight = 0]
        pub fn breed(origin, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) {
            let sender = ensure_signed(origin.clone())?;
            let amount = T::KittyReserveFunds::get();

            let new_kitty_id = Self::do_breed(&sender, kitty_id_1, kitty_id_2)?;
            // 质押token
            KittyLockAmount::<T>::insert(&new_kitty_id, amount.clone());

            Self::reserve_funds(origin, sender.clone(), amount)?;

            // 更新kitty node children关系
            let mut node_vec = KittyNodeStorage::<T>::take();
            for k in &mut node_vec.iter_mut() {
                if k._self == kitty_id_1 {
                    k.children.push(new_kitty_id);
                } else if k._self == kitty_id_2 {
                    k.children.push(new_kitty_id);
                }
            }

            let node = KittyNode {
                _self: new_kitty_id,
                children: Vec::new(),
                companion: Some((kitty_id_1, kitty_id_2)),
            };

            node_vec.push(node);
            // 更新kittynode 关系vec
            KittyNodeStorage::<T>::put(node_vec);
            
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

    // 随机数
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