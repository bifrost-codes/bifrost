// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
use frame_support::pallet_prelude::*;
use frame_support::traits::Get;
use frame_support::traits::ValidatorRegistration;
use frame_support::sp_runtime::SaturatedConversion;
use frame_system::pallet_prelude::*;
use pallet_session::SessionManager;
use sp_std::prelude::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	// Import various types used to declare pallet in scope.
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type ValidatorRegistrationChecker: ValidatorRegistration<Self::AccountId>;
		type EpochDuration: Get<u64>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ValidatorAdded(T::AccountId),
		ValidatorRemoved(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		NotRegistered,
		NotValidator,
	}

	#[pallet::storage]
	#[pallet::getter(fn validators)]
	pub(super) type Validators<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn is_changed)]
	pub type IsChanged<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_validators: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				initial_validators: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for val_id in &self.initial_validators {
				<Validators<T>>::insert(val_id, true);
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_: T::BlockNumber) {}

		fn on_initialize(_: T::BlockNumber) -> Weight {
			0
		}

		fn offchain_worker(_: T::BlockNumber) {}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub(super) fn add_validator(
			origin: OriginFor<T>,
			validator_id: T::AccountId
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			ensure!(
				T::ValidatorRegistrationChecker::is_registered(&validator_id),
				<Error<T>>::NotRegistered
			);
			<Validators<T>>::insert(&validator_id, true);

			IsChanged::<T>::put(true);

			Self::deposit_event(Event::ValidatorAdded(validator_id));

			Ok(().into())
		}

		#[pallet::weight(0)]
		pub(super) fn remove_validator(
			origin: OriginFor<T>,
			validator_id: T::AccountId
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			ensure!(
				<Validators<T>>::contains_key(&validator_id),
				<Error<T>>::NotValidator
			);
			<Validators<T>>::remove(&validator_id);

			IsChanged::<T>::put(true);

			Self::deposit_event(Event::ValidatorRemoved(validator_id));

			Ok(().into())
		}
	}

	impl<T: Config> SessionManager<T::AccountId> for Pallet<T> {
		fn new_session(_: u32) -> Option<Vec<T::AccountId>> {
			let result = if IsChanged::<T>::get() {
				Some(<Validators<T>>::iter().map(|(val_id, _)| val_id).collect())
			} else {
				None
			};

			IsChanged::<T>::put(false);

			result
		}

		fn end_session(_: u32) {}

		fn start_session(_: u32) {}
	}
}
