#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use sp_std::boxed::Box;
use sp_std::vec::Vec;
use sp_runtime::{
	SaturatedConversion,
	traits::{
		Saturating, Zero
	}
};

use frame_system::pallet_prelude::*;
use frame_support::{
	pallet_prelude::*,
	RuntimeDebug,
	ensure,
	traits::{
		Currency, LockableCurrency, ReservableCurrency,
		UnfilteredDispatchable, EstimateCallFee
	},
	weights:: {
		GetDispatchInfo,
		// Weight
	}
};

use scale_info::TypeInfo;
use codec::{Decode, Encode};


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct StakingLevel<Balance> {
	bic_locked: Balance,
	bandwidth: Balance
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	
	#[pallet::config]
	pub trait Config: frame_system::Config + Sized {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Call: Parameter + UnfilteredDispatchable<Origin = Self::Origin> + GetDispatchInfo;

		type Currency: ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
		
		#[pallet::constant]
		type Period: Get<Self::BlockNumber>; //TODO: assigned in runtime

		type TxPayment: EstimateCallFee<<Self as Config>::Call, BalanceOf<Self>>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
	
		#[pallet::weight(10_000)]
		pub fn stake_bic(
			origin: OriginFor<T>,
			#[pallet::compact] amount: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			
			let current_stake = Self::get_stake(&sender);
			let now_stake = current_stake.saturating_add(amount);
			
			StakingMap::<T>::insert(&sender, now_stake);
			T::Currency::reserve(&sender, amount)?;

			Ok(().into())
		}

		#[pallet::weight(10_000)]
		pub fn unstake_bic(
			origin: OriginFor<T>,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			ensure!(StakingMap::<T>::contains_key(&sender), Error::<T>::SomthingErr); //TODO: clarify error

			let current_stake = Self::get_stake(&sender);
			T::Currency::unreserve(&sender, current_stake);

			StakingMap::<T>::remove(&sender);
			BandwidthMap::<T>::remove(&sender);

			Ok(().into())
		}


		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			(dispatch_info.weight.saturating_add(10_000), dispatch_info.class)
		})]
		pub fn feeless_call(
			origin: OriginFor<T>,
			call: Box<<T as Config>::Call>,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin.clone())?;
			
			let call_fee = T::TxPayment::estimate_call_fee(&call, ().into());
			let remain_bandwidth = Self::get_bandwidth(&sender);
			ensure!(remain_bandwidth >= call_fee, Error::<T>::SomthingErr);
			BandwidthMap::<T>::insert(&sender, remain_bandwidth.saturating_sub(call_fee));

			call.dispatch_bypass_filter(origin)?;
			Ok(Pays::No.into())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {

		fn on_finalize(n: T::BlockNumber) {
			Self::finalize_block(n);
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn get_bandwidth)]
	pub(super) type BandwidthMap<T: Config> = StorageMap<
	    _,
	    Blake2_128Concat,
	    T::AccountId,
	    BalanceOf<T>,
	    ValueQuery
    	>;

	#[pallet::storage]
	#[pallet::getter(fn get_stake)]
	pub(super) type StakingMap<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BalanceOf<T>,
		ValueQuery
		>;
	
	#[pallet::storage]
	#[pallet::getter(fn get_staking_level)]
	pub(super) type StakingLevelMap<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u8,
		StakingLevel<BalanceOf<T>>,
		ValueQuery
		>;
	
	//TODO: define this
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		SomethingStored(u32, T::AccountId),
	}

	//TODO: define this
	#[pallet::error]
	pub enum Error<T> {
		SomthingErr
	}
	
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		_phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::add_staking_level(
				1,
				BalanceOf::<T>::saturated_from(5e18 as u128), 
				BalanceOf::<T>::saturated_from(1e18 as u128)
			);

			Pallet::<T>::add_staking_level(
				2, 
				BalanceOf::<T>::saturated_from(1e19 as u128), 
				BalanceOf::<T>::saturated_from(2e18 as u128)
			);
		}
	}
}

impl<T: Config> Pallet<T> {
	fn add_staking_level(level_index: u8, bic_locked: BalanceOf<T>, bandwidth: BalanceOf<T>) {
		StakingLevelMap::<T>::insert(
			level_index,
			StakingLevel {
				bic_locked,
				bandwidth
			}
		);
	}

	fn init_stake_new_period() {
		let mut level_keys = StakingLevelMap::<T>::iter_keys().collect::<Vec<_>>();
		level_keys.sort();
		level_keys.reverse();
		let staking_account_keys = StakingMap::<T>::iter_keys().collect::<Vec<_>>();

		for account in staking_account_keys.iter() {
			let account_staking = Self::get_stake(&account);

			for level in level_keys.iter() {
				let staking_level = Self::get_staking_level(level);
				let bic_locked = staking_level.bic_locked;
				let bandwidth = staking_level.bandwidth;

				if account_staking >= bic_locked {
					BandwidthMap::<T>::insert(&account, bandwidth);
					break;
				}
			}
		}

	}

	fn finalize_block(now: T::BlockNumber) {
		if !(now % T::Period::get()).is_zero() {
			return;
		}
		Self::init_stake_new_period();
	}
}