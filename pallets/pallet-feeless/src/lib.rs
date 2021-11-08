#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use frame_system::pallet_prelude::*;
use sp_std::boxed::Box;
use sp_runtime::SaturatedConversion;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::DispatchResultWithPostInfo, 
		pallet_prelude::*,
		traits::UnfilteredDispatchable, 
		weights::{
			GetDispatchInfo,
			ClassifyDispatch,
			PaysFee,
			WeighData
		}
	};
	use sp_runtime::traits::{
		AtLeast32BitUnsigned,
		Saturating
	};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Balance: Member + Parameter + AtLeast32BitUnsigned + Default + Copy + From<u64> + MaxEncodedLen;

		type Call: Parameter + UnfilteredDispatchable<Origin = Self::Origin> + GetDispatchInfo;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	{

		#[pallet::weight(10_000)]
		pub fn grant_bandwidth(
			origin: OriginFor<T>,
			user: T::AccountId,
			#[pallet::compact] amount: T::Balance
		) -> DispatchResultWithPostInfo {
			// This is a public call, so we ensure that the origin is some signed account.
			let sender = ensure_signed(origin)?;
			ensure!(sender == Self::key(), Error::<T>::SomthingErr);
			
			BandwidthAllow::<T>::insert(user, amount);

			// Sudo user does not pay a fee.
			Ok(Pays::No.into())
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
			
			let dispatch_info = call.get_dispatch_info();
			let remain_bandwidth = Self::get_balance(&sender);
			
			let res = call.dispatch_bypass_filter(origin);

			let tx_weight = T::Balance::from(dispatch_info.weight);
			if remain_bandwidth < tx_weight {
				BandwidthAllow::<T>::insert(
					&sender, 
					remain_bandwidth.saturating_sub(remain_bandwidth)
				);
				let need_to_pay = tx_weight.saturating_sub(remain_bandwidth).saturated_into::<u64>();
				return Ok(Some(need_to_pay).into())
			} else {
				BandwidthAllow::<T>::insert(
					&sender, 
					remain_bandwidth.saturating_sub(tx_weight)
				);
				return Ok(Pays::No.into())
			}
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn get_balance)]
	pub(super) type BandwidthAllow<T: Config> = StorageMap<
	    _,
	    Blake2_128Concat,
	    T::AccountId,
	    T::Balance,
	    ValueQuery
    	>;

	#[pallet::storage]
	#[pallet::getter(fn key)]
	pub(super) type Key<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

	
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		SomethingStored(u32, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		SomthingErr
	}

	
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		/// The `AccountId` of the sudo key.
		pub key: T::AccountId,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { key: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<Key<T>>::put(&self.key);
		}
	}
}
