// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use node_primitives::{traits::BancorHandler, CurrencyId, CurrencyIdExt};
use num_bigint::BigUint;
use orml_traits::MultiCurrency;
use sp_arithmetic::per_things::{PerThing, Perbill, Percent};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Saturating, Zero},
	SaturatedConversion,
};
use weights::WeightInfo;

mod mock;
mod tests;
pub mod weights;

pub use pallet::*;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

const BILLION: u128 = 1_000_000_000;

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub struct BancorPool<Balance> {
	currency_id: CurrencyId,      // ksm, dot, etc.
	token_pool: Balance,          // token supply of the pool
	vstoken_pool: Balance,        // vstoken balance of the pool
	token_ceiling: Balance,       // token available for sale
	token_base_supply: Balance,   // initial virtual supply of token for the pool
	vstoken_base_supply: Balance, // initial virtual balance of vstoken for the pool
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		#[pallet::constant]
		type InterventionPercentage: Get<Percent>;

		/// Set default weight.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		CurrencyIdNotExist,
		AmountNotGreaterThanZero,
		BancorPoolNotExist,
		ConversionError,
		TokenSupplyNotEnough,
		VSTokenSupplyNotEnough,
		PriceNotQualified,
		CalculationOverflow,
		NotSupportTokenType,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Token has been sold.
		///
		/// [buyer, currencyId, token_sold, vsToken_paid]
		TokenSold(AccountIdOf<T>, CurrencyId, BalanceOf<T>, BalanceOf<T>),
		/// [buyer, currencyId, vsToken_sold, Token_paid]
		VSTokenSold(AccountIdOf<T>, CurrencyId, BalanceOf<T>, BalanceOf<T>),
	}

	// key is token, value is BancorPool struct.
	#[pallet::storage]
	#[pallet::getter(fn get_bancor_pool)]
	pub type BancorPools<T> = StorageMap<_, Blake2_128Concat, CurrencyId, BancorPool<BalanceOf<T>>>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub bancor_pools: Vec<(CurrencyId, BalanceOf<T>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { bancor_pools: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (currency_id, base_balance) in self.bancor_pools.iter() {
				let pool = BancorPool {
					currency_id: *currency_id,
					token_pool: Zero::zero(),
					vstoken_pool: Zero::zero(),
					token_ceiling: Zero::zero(),
					token_base_supply: base_balance.saturating_mul(BalanceOf::<T>::from(2u32)),
					vstoken_base_supply: *base_balance,
				};

				BancorPools::<T>::insert(currency_id, pool);
			}
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::add_token_to_pool())]
		pub fn add_token_to_pool(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			token_amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(currency_id.is_token(), Error::<T>::NotSupportTokenType);

			Self::add_token(currency_id, token_amount)?;

			Ok(())
		}

		// exchange vstoken for token
		#[pallet::weight(T::WeightInfo::exchange_for_token())]
		pub fn exchange_for_token(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			vstoken_amount: BalanceOf<T>,
			token_out_min: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			ensure!(currency_id.is_token(), Error::<T>::NotSupportTokenType);
			let vstoken_id = currency_id.to_vstoken().map_err(|_| Error::<T>::ConversionError)?;

			// Get exchanger's vstoken balance
			let vstoken_balance = T::MultiCurrency::free_balance(vstoken_id, &exchanger);
			ensure!(vstoken_balance >= vstoken_amount, Error::<T>::NotEnoughBalance);

			// make changes in the bancor pool
			let token_amount = Self::calculate_price_for_token(currency_id, vstoken_amount)?;

			ensure!(token_amount >= token_out_min, Error::<T>::PriceNotQualified);

			Self::revise_bancor_pool_vstoken_buy_token(currency_id, token_amount, vstoken_amount)?;

			// make changes in account balance
			T::MultiCurrency::withdraw(vstoken_id, &exchanger, vstoken_amount)?;
			T::MultiCurrency::deposit(currency_id, &exchanger, token_amount)?;

			Self::deposit_event(Event::TokenSold(
				exchanger,
				currency_id,
				token_amount,
				vstoken_amount,
			));

			Ok(().into())
		}

		// exchange token for vstoken
		#[pallet::weight(T::WeightInfo::exchange_for_vstoken())]
		pub fn exchange_for_vstoken(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			token_amount: BalanceOf<T>,
			vstoken_out_min: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			ensure!(currency_id.is_token(), Error::<T>::NotSupportTokenType);
			let vstoken_id = currency_id.to_vstoken().map_err(|_| Error::<T>::ConversionError)?;

			// Get exchanger's token balance
			let token_balance = T::MultiCurrency::free_balance(currency_id, &exchanger);
			ensure!(token_balance >= token_amount, Error::<T>::NotEnoughBalance);

			// make changes in the bancor pool
			let vstoken_amount = Self::calculate_price_for_vstoken(currency_id, token_amount)?;

			ensure!(vstoken_amount >= vstoken_out_min, Error::<T>::PriceNotQualified);

			Self::revise_bancor_pool_token_buy_vstoken(currency_id, token_amount, vstoken_amount)?;

			// make changes in account balance
			T::MultiCurrency::withdraw(currency_id, &exchanger, token_amount)?;
			T::MultiCurrency::deposit(vstoken_id, &exchanger, vstoken_amount)?;

			Self::deposit_event(Event::VSTokenSold(
				exchanger,
				currency_id,
				vstoken_amount,
				token_amount,
			));

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Formula: Supply * ((1 + vsDOT/Balance) ^CW -1)
	/// Supply: The total amount of DOT currently Sent in plus initiated virtual amount of DOT
	/// Balance: The total amount of vsDOT currently Sent in plus initiated virtual amount of vsDOT
	/// CW: Constant, here is 1/2
	pub fn calculate_price_for_token(
		token_id: CurrencyId,
		vstoken_amount: BalanceOf<T>,
	) -> Result<BalanceOf<T>, Error<T>> {
		// ensure!(token_id.exist(), Error::<T>::CurrencyIdNotExist);
		ensure!(vstoken_amount > Zero::zero(), Error::<T>::AmountNotGreaterThanZero);

		let pool_info = Self::get_bancor_pool(token_id).ok_or(Error::<T>::BancorPoolNotExist)?;
		// Only if token_ceiling is not zero, then exchangers can exchange vstokens for tokens.
		ensure!(pool_info.token_ceiling > Zero::zero(), Error::<T>::TokenSupplyNotEnough);

		let (token_supply, vstoken_supply) = (
			pool_info.token_base_supply.saturating_add(pool_info.token_pool),
			pool_info.vstoken_base_supply.saturating_add(pool_info.vstoken_pool),
		);
		ensure!(vstoken_supply > Zero::zero(), Error::<T>::AmountNotGreaterThanZero);

		// To avoid overflow, we introduce num-bigint package.
		let vstoken_amount = {
			let temp: u128 = vstoken_amount.saturated_into();
			BigUint::from(temp)
		};
		let vstoken_supply = {
			let temp: u128 = vstoken_supply.saturated_into();
			BigUint::from(temp)
		};
		let token_supply = {
			let temp: u128 = token_supply.saturated_into();
			BigUint::from(temp)
		};
		let token_supply_square =
			token_supply.checked_mul(&token_supply).ok_or(Error::<T>::CalculationOverflow)?;

		let nominator_lhs = token_supply_square
			.checked_mul(&vstoken_supply)
			.ok_or(Error::<T>::CalculationOverflow)?;
		let nominator_rhs = token_supply_square
			.checked_mul(&vstoken_amount)
			.ok_or(Error::<T>::CalculationOverflow)?;
		let nominator = nominator_lhs
			.checked_add(&nominator_rhs)
			.ok_or(Error::<T>::CalculationOverflow)?;

		let inside =
			nominator.checked_div(&vstoken_supply).ok_or(Error::<T>::CalculationOverflow)?;
		let square_root = inside.nth_root(2);
		let result =
			square_root.checked_sub(&token_supply).ok_or(Error::<T>::CalculationOverflow)?;
		let result_convert: u128 = u128::from_str_radix(&result.to_str_radix(10), 10)
			.map_err(|_| Error::<T>::ConversionError)?;

		let price = BalanceOf::<T>::saturated_from(result_convert);

		// We can not exchange for more than that the the pool has
		ensure!(price <= pool_info.token_ceiling, Error::<T>::TokenSupplyNotEnough);

		Ok(price)
	}

	/// Formula: Balance * (1 - (1 - DOT/Supply)^ (1/CW))
	/// Supply: The total amount of DOT currently Sent in plus initiated virtual amount of DOT
	/// Balance: The total amount of vsDOT currently Sent in plus initiated virtual amount of vsDOT
	/// CW: Constant, here is 1/2
	pub fn calculate_price_for_vstoken(
		token_id: CurrencyId,
		token_amount: BalanceOf<T>,
	) -> Result<BalanceOf<T>, Error<T>> {
		// ensure!(token_id.exist(), Error::<T>::CurrencyIdNotExist);
		ensure!(token_amount > Zero::zero(), Error::<T>::AmountNotGreaterThanZero);

		let pool_info = Self::get_bancor_pool(token_id).ok_or(Error::<T>::BancorPoolNotExist)?;
		ensure!(pool_info.vstoken_pool > Zero::zero(), Error::<T>::VSTokenSupplyNotEnough);

		let (token_supply, vstoken_supply) = (
			pool_info.token_base_supply.saturating_add(pool_info.token_pool),
			pool_info.vstoken_base_supply.saturating_add(pool_info.vstoken_pool),
		);

		// Since token_amount will be deducted from the total token_supply, token_amount should be
		// less than or equal to token_supply.
		ensure!(token_amount <= token_supply, Error::<T>::TokenSupplyNotEnough);
		let mid_item: Perbill =
			PerThing::from_rational(token_supply.saturating_sub(token_amount), token_supply);
		let square_item: Perbill = mid_item.square();

		// Destruct the nominator from permill and divide the result by the denominator of a
		// million.
		let rhs = Perbill::one().saturating_sub(square_item);
		let rhs_nominator = BalanceOf::<T>::saturated_from(rhs.deconstruct());
		let price =
			rhs_nominator.saturating_mul(vstoken_supply) / BalanceOf::<T>::saturated_from(BILLION);

		// We can not exchange for more than that the the pool has
		ensure!(price <= pool_info.vstoken_pool, Error::<T>::VSTokenSupplyNotEnough);

		Ok(price)
	}

	/// one vstoken worths how many tokens
	// formula: token_supply/ (vstoken_balance/ cw). Note: cw = 1/2
	// return value: (nominator, denominator)
	pub fn get_instant_vstoken_price(
		currency_id: CurrencyId,
	) -> Result<(BalanceOf<T>, BalanceOf<T>), Error<T>> {
		let pool_info = Self::get_bancor_pool(currency_id).ok_or(Error::<T>::BancorPoolNotExist)?;
		let (token_supply, vstoken_supply) = (
			pool_info.token_base_supply.saturating_add(pool_info.token_pool),
			pool_info.vstoken_base_supply.saturating_add(pool_info.vstoken_pool),
		);

		Ok((token_supply, BalanceOf::<T>::saturated_from(2u128).saturating_mul(vstoken_supply)))
	}

	// one token worths how many vstokens
	pub fn get_instant_token_price(
		currency_id: CurrencyId,
	) -> Result<(BalanceOf<T>, BalanceOf<T>), Error<T>> {
		let pool_info = Self::get_bancor_pool(currency_id).ok_or(Error::<T>::BancorPoolNotExist)?;
		let (token_supply, vstoken_supply) = (
			pool_info.token_base_supply.saturating_add(pool_info.token_pool),
			pool_info.vstoken_base_supply.saturating_add(pool_info.vstoken_pool),
		);

		Ok((BalanceOf::<T>::saturated_from(2u128).saturating_mul(vstoken_supply), token_supply))
	}

	pub(crate) fn increase_bancor_pool_ceiling(
		currency_id: CurrencyId,
		increase_amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		BancorPools::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			match pool {
				Some(pool_info) => {
					pool_info.token_ceiling =
						pool_info.token_ceiling.saturating_add(increase_amount);
					Ok(())
				}
				_ => Err(Error::<T>::BancorPoolNotExist),
			}
		})?;

		Ok(())
	}

	pub(crate) fn revise_bancor_pool_token_buy_vstoken(
		currency_id: CurrencyId,
		token_amount: BalanceOf<T>,
		vstoken_amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		BancorPools::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			match pool {
				Some(pool_info) => {
					ensure!(pool_info.token_pool >= token_amount, Error::<T>::TokenSupplyNotEnough);
					ensure!(
						pool_info.vstoken_pool >= vstoken_amount,
						Error::<T>::VSTokenSupplyNotEnough
					);
					pool_info.token_pool = pool_info.token_pool.saturating_sub(token_amount);
					pool_info.vstoken_pool = pool_info.vstoken_pool.saturating_sub(vstoken_amount);
					Ok(())
				}
				_ => Err(Error::<T>::BancorPoolNotExist),
			}
		})?;

		Ok(())
	}

	pub(crate) fn revise_bancor_pool_vstoken_buy_token(
		currency_id: CurrencyId,
		token_amount: BalanceOf<T>,
		vstoken_amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		BancorPools::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			match pool {
				Some(pool_info) => {
					ensure!(
						pool_info.token_ceiling >= token_amount,
						Error::<T>::TokenSupplyNotEnough
					);
					pool_info.token_ceiling = pool_info.token_ceiling.saturating_sub(token_amount);
					pool_info.token_pool = pool_info.token_pool.saturating_add(token_amount);
					pool_info.vstoken_pool = pool_info.vstoken_pool.saturating_add(vstoken_amount);
					Ok(())
				}
				_ => Err(Error::<T>::BancorPoolNotExist),
			}
		})?;

		Ok(())
	}
}

impl<T: Config> BancorHandler<BalanceOf<T>> for Pallet<T> {
	//  check whether the price of vstoken (token/vstoken) is lower than 75%. if yes, then half of
	// this newly released token should be used to buy vstoken,  so that the price of vstoken will
	// increase. Meanwhile, the other half will be put on the ceiling variable to indicate exchange
	// availability. 	If not, all the newly release token should be put aside to the ceiling to not
	// to impact the pool price.
	fn add_token(currency_id: CurrencyId, token_amount: BalanceOf<T>) -> Result<(), DispatchError> {
		// get the current price of vstoken
		let (nominator, denominator) = Self::get_instant_vstoken_price(currency_id)?;

		let amount_kept: BalanceOf<T>;
		// if vstoken price is lower than 0.75 token
		if T::InterventionPercentage::get().saturating_reciprocal_mul_floor(nominator) <=
			denominator
		{
			amount_kept = token_amount / BalanceOf::<T>::saturated_from(2u128);
		} else {
			amount_kept = token_amount;
		}

		let sell_amount = token_amount.saturating_sub(amount_kept);

		// deal with ceiling variable
		if amount_kept != Zero::zero() {
			Self::increase_bancor_pool_ceiling(currency_id, amount_kept)?;
		}

		// deal with exchange transaction
		if sell_amount != Zero::zero() {
			// make changes in the bancor pool
			let vstoken_amount = Self::calculate_price_for_vstoken(currency_id, sell_amount)?;
			let sell_result = Self::revise_bancor_pool_token_buy_vstoken(
				currency_id,
				sell_amount,
				vstoken_amount,
			);

			// if somehow not able to sell token, then add the amount to ceiling.
			if let Err(err_msg) = sell_result {
				match err_msg {
					Error::<T>::BancorPoolNotExist => Err(Error::<T>::BancorPoolNotExist),
					_ => Self::increase_bancor_pool_ceiling(currency_id, sell_amount),
				}?;
			}
		}

		Ok(())
	}
}
