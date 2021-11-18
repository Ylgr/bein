#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use codec::Encode;
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{Currency, IsType, ReservableCurrency},
	transactional,
	weights::Weight,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use pallet_evm::AddressMapping;
use sp_core::{crypto::AccountId32, ecdsa};
use sp_io::{
	crypto::secp256k1_ecdsa_recover,
	hashing::keccak_256,
};
use sp_runtime::{
	DispatchResult,
};
use sp_std::vec::Vec;


pub trait MergeAccount<AccountId> {
	fn merge_account(source: &AccountId, dest: &AccountId) -> DispatchResult;
}

pub trait WeightInfo {
	fn claim_account() -> Weight;
}

pub type EcdsaSignature = ecdsa::Signature;
pub type EvmAddress = sp_core::H160;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency for managing Evm account assets.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Mapping from address to account id.
		type AddressMapping: AddressMapping<Self::AccountId>;

		/// Merge free balance from source to dest.
		type MergeAccount: MergeAccount<Self::AccountId>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Mapping between Substrate accounts and EVM accounts
		/// claim account. \[account_id, evm_address\]
		ClaimAccount(T::AccountId, EvmAddress),
	}

	/// Error for evm accounts module.
	#[pallet::error]
	pub enum Error<T> {
		/// AccountId has mapped
		AccountIdHasMapped,
		/// Eth address has mapped
		EthAddressHasMapped,
		/// Bad signature
		BadSignature,
		/// Invalid signature
		InvalidSignature,
		/// Account ref count is not zero
		NonZeroRefCount,
		/// Account still has active reserved
		StillHasActiveReserved,
	}

	#[pallet::storage]
	#[pallet::getter(fn accounts)]
	pub type Accounts<T: Config> = StorageMap<_, Twox64Concat, EvmAddress, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn evm_addresses)]
	pub type EvmAddresses<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, EvmAddress>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Claim account mapping between Substrate accounts and EVM accounts.
		/// Ensure eth_address has not been mapped.
		#[pallet::weight(T::WeightInfo::claim_account())]
		#[transactional]
		pub fn claim_account(
			origin: OriginFor<T>,
			eth_address: EvmAddress,
			eth_signature: EcdsaSignature,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			// ensure account_id and eth_address has not been mapped
			ensure!(!EvmAddresses::<T>::contains_key(&who), Error::<T>::AccountIdHasMapped);
			ensure!(
				!Accounts::<T>::contains_key(eth_address),
				Error::<T>::EthAddressHasMapped
			);

			// recover evm address from signature
			let address = Self::eth_recover(&eth_signature, &who.using_encoded(to_ascii_hex), &[][..])
				.ok_or(Error::<T>::BadSignature)?;
			ensure!(eth_address == address, Error::<T>::InvalidSignature);

			// check if the evm padded address already exists
			let account_id = T::AddressMapping::into_account_id(eth_address);
			if frame_system::Pallet::<T>::account_exists(&account_id) {
				// merge balance from `evm padded address` to `origin`
				T::MergeAccount::merge_account(&account_id, &who)?;
			}

			Accounts::<T>::insert(eth_address, &who);
			EvmAddresses::<T>::insert(&who, eth_address);

			Self::deposit_event(Event::ClaimAccount(who, eth_address));

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	// Constructs the message that Ethereum RPC's `personal_sign` and `eth_sign`
	// would sign.
	pub fn ethereum_signable_message(what: &[u8], extra: &[u8]) -> Vec<u8> {
		let prefix = b"bein evm:";
		let mut l = prefix.len() + what.len() + extra.len();
		let mut rev = Vec::new();
		while l > 0 {
			rev.push(b'0' + (l % 10) as u8);
			l /= 10;
		}
		let mut v = b"\x19Ethereum Signed Message:\n".to_vec();
		v.extend(rev.into_iter().rev());
		v.extend_from_slice(&prefix[..]);
		v.extend_from_slice(what);
		v.extend_from_slice(extra);
		v
	}

	// Attempts to recover the Ethereum address from a message signature signed by
	// using the Ethereum RPC's `personal_sign` and `eth_sign`.
	pub fn eth_recover(s: &EcdsaSignature, what: &[u8], extra: &[u8]) -> Option<EvmAddress> {
		let msg = keccak_256(&Self::ethereum_signable_message(what, extra));
		let mut res = EvmAddress::default();
		res.0
			.copy_from_slice(&keccak_256(&secp256k1_ecdsa_recover(s.as_ref(), &msg).ok()?[..])[12..]);
		Some(res)
	}

	pub fn eth_public(secret: &secp256k1::SecretKey) -> secp256k1::PublicKey {
		secp256k1::PublicKey::from_secret_key(secret)
	}
	pub fn eth_address(secret: &secp256k1::SecretKey) -> EvmAddress {
		EvmAddress::from_slice(&keccak_256(&Self::eth_public(secret).serialize()[1..65])[12..])
	}
	pub fn eth_sign(secret: &secp256k1::SecretKey, what: &[u8], extra: &[u8]) -> EcdsaSignature {
		let msg = keccak_256(&Self::ethereum_signable_message(&to_ascii_hex(what)[..], extra));
		let (sig, recovery_id) = secp256k1::sign(&secp256k1::Message::parse(&msg), secret);
		let mut r = [0u8; 65];
		r[0..64].copy_from_slice(&sig.serialize()[..]);
		r[64] = recovery_id.serialize();
		EcdsaSignature::from_slice(&r)
	}
}

pub struct EvmAddressMapping<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> AddressMapping<T::AccountId> for EvmAddressMapping<T>
	where
		T::AccountId: IsType<AccountId32>,
{
	fn into_account_id(address: EvmAddress) -> T::AccountId {
		if let Some(acc) = Accounts::<T>::get(address) {
			acc
		} else {
			let mut data: [u8; 32] = [0u8; 32];
			data[0..4].copy_from_slice(b"evm:");
			data[4..24].copy_from_slice(&address[..]);
			AccountId32::from(data).into()
		}
	}
}

/// Converts the given binary data into ASCII-encoded hex. It will be twice
/// the length.
pub fn to_ascii_hex(data: &[u8]) -> Vec<u8> {
	let mut r = Vec::with_capacity(data.len() * 2);
	let mut push_nibble = |n| r.push(if n < 10 { b'0' + n } else { b'a' - 10 + n });
	for &b in data.iter() {
		push_nibble(b / 16);
		push_nibble(b % 16);
	}
	r
}
