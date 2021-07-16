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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// Re-export pallet items so that they can be accessed from the crate namespace.
use frame_support::{pallet_prelude::*, sp_runtime::MultiSignature};
use node_primitives::ParaId;
use orml_traits::MultiCurrency;
pub use pallet::*;

type TrieIndex = u32;

pub trait WeightInfo {
	fn create() -> Weight;
	fn contribute() -> Weight;
	fn on_finalize(n: u32) -> Weight;
}

pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
	fn create() -> Weight {
		0
	}

	fn contribute() -> Weight {
		0
	}

	fn on_finalize(_n: u32) -> Weight {
		0
	}
}

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum FundStatus {
	Ongoing,
	Retired,
	Success,
	Failed,
	RefundWithdrew,
	RedeemWithdrew,
	End,
}

impl Default for FundStatus {
	fn default() -> Self {
		FundStatus::Ongoing
	}
}

/// Information on a funding effort for a pre-existing parachain. We assume that the parachain
/// ID is known as it's used for the key of the storage item for which this is the value
/// (`Funds`).
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
#[codec(dumb_trait_bound)]
pub struct FundInfo<AccountId, Balance, LeasePeriod> {
	/// The owning account who placed the deposit.
	depositor: AccountId,
	/// The amount of deposit placed.
	deposit: Balance,
	/// The total amount raised.
	raised: Balance,
	/// A hard-cap on the amount that may be contributed.
	cap: Balance,
	/// First slot in range to bid on; it's actually a LeasePeriod, but that's the same type as
	/// BlockNumber.
	first_slot: LeasePeriod,
	/// Last slot in range to bid on; it's actually a LeasePeriod, but that's the same type as
	/// BlockNumber.
	last_slot: LeasePeriod,
	/// Index used for the child trie of this fund
	trie_index: TrieIndex,
	/// Fund status
	status: FundStatus,
}

#[derive(Encode, Decode)]
pub enum CrowdloanContributeCall<BalanceOf> {
	#[codec(index = 73)]
	CrowdloanContribute(ContributeCall<BalanceOf>),
}

#[derive(Encode, Decode)]
pub enum CrowdloanWithdrawCall<AccountIdOf> {
	#[codec(index = 73)]
	CrowdloanWithdraw(WithdrawCall<AccountIdOf>),
}

#[derive(Debug, PartialEq, Encode, Decode)]
pub struct Contribution<BalanceOf> {
	#[codec(compact)]
	index: ParaId,
	#[codec(compact)]
	value: BalanceOf,
	signature: Option<MultiSignature>,
}

#[derive(Encode, Decode)]
pub enum ContributeCall<BalanceOf> {
	#[codec(index = 1)]
	Contribute(Contribution<BalanceOf>),
}

#[derive(Debug, PartialEq, Encode, Decode)]
pub struct Withdraw<AccountIdOf> {
	who: AccountIdOf,
	#[codec(compact)]
	index: ParaId,
}

#[derive(Encode, Decode)]
pub enum WithdrawCall<AccountIdOf> {
	#[codec(index = 2)]
	Withdraw(Withdraw<AccountIdOf>),
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, Copy)]
pub enum ContributionStatus<BalanceOf> {
	Idle,
	Refunded,
	Unlocked,
	Refunding,
	Contributing(BalanceOf),
}

impl<BalanceOf> ContributionStatus<BalanceOf>
where
	BalanceOf: frame_support::sp_runtime::traits::Zero + Clone + Copy,
{
	pub fn is_contributing(&self) -> bool {
		match self {
			Self::Contributing(_) => true,
			_ => false,
		}
	}

	pub fn contributing(&self) -> BalanceOf {
		match self {
			Self::Contributing(contributing) => *contributing,
			_ => frame_support::sp_runtime::traits::Zero::zero(),
		}
	}
}

impl<BalanceOf> Default for ContributionStatus<BalanceOf> {
	fn default() -> Self {
		Self::Idle
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, Copy)]
pub enum RedeemStatus<BalanceOf> {
	Idle,
	Redeeming(BalanceOf),
}

impl<BalanceOf> RedeemStatus<BalanceOf>
where
	BalanceOf: frame_support::sp_runtime::traits::Zero + Clone + Copy,
{
	pub fn is_redeeming(&self) -> bool {
		match self {
			Self::Redeeming(..) => true,
			_ => false,
		}
	}

	pub fn redeeming(&self) -> BalanceOf {
		match self {
			Self::Redeeming(redeeming) => *redeeming,
			_ => frame_support::sp_runtime::traits::Zero::zero(),
		}
	}
}

impl<BalanceOf> Default for RedeemStatus<BalanceOf> {
	fn default() -> Self {
		Self::Idle
	}
}

#[frame_support::pallet]
pub mod pallet {
	// Import various types used to declare pallet in scope.
	use frame_support::{
		pallet_prelude::{storage::child, *},
		sp_runtime::traits::{AccountIdConversion, CheckedAdd, Hash, Saturating, Zero},
		storage::ChildTriePrefixIterator,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use node_primitives::{BancorHandler, CurrencyId, LeasePeriod, ParaId};
	use orml_traits::{currency::TransferAll, MultiCurrency, MultiReservableCurrency};
	use polkadot_parachain::primitives::Id as PolkadotParaId;
	use sp_arithmetic::Percent;
	use sp_std::{convert::TryInto, prelude::*};
	use xcm::v0::{
		prelude::{XcmError, XcmResult},
		Junction, MultiLocation,
	};
	use xcm_support::BifrostXcmExecutor;

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config<BlockNumber = LeasePeriod> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// ModuleID for the crowdloan module. An appropriate value could be
		/// ```ModuleId(*b"py/cfund")```
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The amount to be held on deposit by the depositor of a crowdloan.
		type SubmissionDeposit: Get<BalanceOf<Self>>;

		/// The minimum amount that may be contributed into a crowdloan. Should almost certainly be
		/// at least ExistentialDeposit.
		#[pallet::constant]
		type MinContribution: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type RelayChainToken: Get<CurrencyId>;

		#[pallet::constant]
		type DepositToken: Get<CurrencyId>;

		/// The number of blocks over which a single period lasts.
		#[pallet::constant]
		type LeasePeriod: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type VSBondValidPeriod: Get<BlockNumberFor<Self>>;

		/// The time interval from 1:1 redeem-pool to bancor-pool to release.
		#[pallet::constant]
		type ReleaseCycle: Get<LeasePeriod>;

		/// The release ratio from the 1:1 redeem-pool to the bancor-pool per cycle.
		///
		/// **NOTE: THE RELEASE RATIO MUST BE IN [0, 1].**
		#[pallet::constant]
		type ReleaseRatio: Get<Percent>;

		#[pallet::constant]
		type RemoveKeysLimit: Get<u32>;

		#[pallet::constant]
		type SlotLength: Get<LeasePeriod>;

		type MultiCurrency: TransferAll<AccountIdOf<Self>>
			+ MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type BancorPool: BancorHandler<BalanceOf<Self>>;

		type ExecuteXcmOrigin: EnsureOrigin<
			<Self as frame_system::Config>::Origin,
			Success = MultiLocation,
		>;

		type BifrostXcmExecutor: BifrostXcmExecutor;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Create a new crowdloaning campaign. [fund_index]
		Created(ParaId),
		/// Contributing to a crowd sale. [who, fund_index, amount]
		Contributing(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Contributed to a crowd sale. [who, fund_index, amount]
		Contributed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Fail on contribute to crowd sale. [who, fund_index, amount]
		ContributeFailed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Withdrawing full balance of a contributor. [who, fund_index, amount]
		Withdrawing(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Withdrew full balance of a contributor. [who, fund_index, amount]
		Withdrew(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Fail on withdraw full balance of a contributor. [who, fund_index, amount]
		WithdrawFailed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Refunding to account. [who, fund_index, amount]
		Refunding(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Refunded to account. [who, fund_index, amount]
		Refunded(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Fail on refund to account. [who,fund_index, amount]
		RefundFailed(AccountIdOf<T>, ParaId, BalanceOf<T>),
		/// Redeeming to account. [who, fund_index, first_slot, last_slot, value]
		Redeeming(AccountIdOf<T>, ParaId, LeasePeriod, LeasePeriod, BalanceOf<T>),
		/// Redeemed to account. [who, fund_index, first_slot, last_slot, value]
		Redeemed(AccountIdOf<T>, ParaId, LeasePeriod, LeasePeriod, BalanceOf<T>),
		/// Fail on redeem to account. [who, fund_index, first_slot, last_slot, value]
		RedeemFailed(AccountIdOf<T>, ParaId, LeasePeriod, LeasePeriod, BalanceOf<T>),
		/// Fund is dissolved. [fund_index]
		Dissolved(ParaId),
		/// The vsToken/vsBond was be unlocked. [who, fund_index, value]
		Unlocked(AccountIdOf<T>, ParaId, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The first slot needs to at least be less than 3 `max_value`.
		FirstSlotTooFarInFuture,
		/// Last slot must be greater than first slot.
		LastSlotBeforeFirstSlot,
		/// The last slot cannot be more then 3 slots after the first slot.
		LastSlotTooFarInFuture,
		/// There was an overflow.
		Overflow,
		/// The contribution was below the minimum, `MinContribution`.
		ContributionTooSmall,
		/// The account doesn't have any contribution to the fund.
		ZeroContribution,
		/// Invalid fund index.
		InvalidParaId,
		/// Invalid fund status.
		InvalidFundStatus,
		/// Invalid contribution status.
		InvalidContributionStatus,
		/// Contributions exceed maximum amount.
		CapExceeded,
		/// The origin of this call is invalid.
		UnauthorizedAccount,
		/// The fund has been registered.
		FundAlreadyCreated,
		/// Crosschain xcm failed
		XcmFailed,
		/// Don't have enough vsToken/vsBond to refund
		NotEnoughReservedAssetsToRefund,
		/// Don't have enough token to refund by users
		NotEnoughBalanceInRefundPool,
		/// Don't have enough vsToken/vsBond to unlock
		NotEnoughBalanceToUnlock,
		/// The vsBond is expired now
		VSBondExpired,
		/// The vsBond cannot be redeemed by now
		UnRedeemableNow,
		/// Dont have enough vsToken/vsBond to redeem
		NotEnoughFreeAssetsToRedeem,
		/// Dont have enough vsToken/vsBond to unlock when redeem failed
		NotEnoughReservedAssetsToUnlockWhenRedeemFailed,
		/// Don't have enough token to redeem by users
		NotEnoughBalanceInRedeemPool,
		/// Invalid redeem status
		InvalidRedeemStatus,
	}

	/// Tracker for the next available trie index
	#[pallet::storage]
	#[pallet::getter(fn current_trie_index)]
	pub(super) type CurrentTrieIndex<T: Config> = StorageValue<_, TrieIndex, ValueQuery>;

	/// Info on all of the funds.
	#[pallet::storage]
	#[pallet::getter(fn funds)]
	pub(super) type Funds<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ParaId,
		Option<FundInfo<AccountIdOf<T>, BalanceOf<T>, LeasePeriod>>,
		ValueQuery,
	>;

	/// The balance can be refunded to users.
	#[pallet::storage]
	#[pallet::getter(fn refund_pool)]
	pub(super) type RefundPool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// The balance can be redeemed to users.
	#[pallet::storage]
	#[pallet::getter(fn redeem_pool)]
	pub(super) type RedeemPool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn redeem_status)]
	pub(super) type RedeemExtras<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		(ParaId, LeasePeriod, LeasePeriod),
		RedeemStatus<BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn fund_success(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			let fund_new = FundInfo { status: FundStatus::Success, ..fund };
			Funds::<T>::insert(index, Some(fund_new));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn fund_fail(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			// crownload is failed, so enable the withdrawal function of vsToken/vsBond
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			let fund_new = FundInfo { status: FundStatus::Failed, ..fund };
			Funds::<T>::insert(index, Some(fund_new));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn fund_retire(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Success, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			let fund_new = FundInfo { status: FundStatus::Retired, ..fund };
			Funds::<T>::insert(index, Some(fund_new));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn fund_end(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::RefundWithdrew ||
					fund.status == FundStatus::RedeemWithdrew,
				Error::<T>::InvalidFundStatus
			);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			let fund_new = FundInfo { status: FundStatus::End, ..fund };
			Funds::<T>::insert(index, Some(fund_new));

			Ok(())
		}

		/// Unlock the reserved vsToken/vsBond after fund success
		#[pallet::weight(0)]
		pub fn unlock(
			_origin: OriginFor<T>,
			who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
		) -> DispatchResult {
			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(
				fund.status == FundStatus::Success ||
					fund.status == FundStatus::Retired ||
					fund.status == FundStatus::RedeemWithdrew ||
					fund.status == FundStatus::End,
				Error::<T>::InvalidFundStatus
			);

			let (contributed, status) = Self::contribution(fund.trie_index, &who);
			ensure!(status == ContributionStatus::Idle, Error::<T>::InvalidContributionStatus);

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);

			let balance = T::MultiCurrency::unreserve(vsToken, &who, contributed);
			ensure!(balance == Zero::zero(), Error::<T>::NotEnoughBalanceToUnlock);
			let balance = T::MultiCurrency::unreserve(vsBond, &who, contributed);
			ensure!(balance == Zero::zero(), Error::<T>::NotEnoughBalanceToUnlock);

			Self::put_contribution(
				fund.trie_index,
				&who,
				contributed,
				ContributionStatus::Unlocked,
			);

			Self::deposit_event(Event::<T>::Unlocked(who, index, contributed));

			Ok(())
		}

		/// TODO: Refactor the docs.
		/// Create a new crowdloaning campaign for a parachain slot deposit for the current auction.
		#[pallet::weight(T::WeightInfo::create())]
		pub fn create(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] cap: BalanceOf<T>,
			#[pallet::compact] first_slot: LeasePeriod,
			#[pallet::compact] last_slot: LeasePeriod,
		) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			ensure!(!Funds::<T>::contains_key(index), Error::<T>::FundAlreadyCreated);

			ensure!(first_slot <= last_slot, Error::<T>::LastSlotBeforeFirstSlot);

			let last_slot_limit = first_slot
				.checked_add(((T::SlotLength::get() as u32) - 1).into())
				.ok_or(Error::<T>::FirstSlotTooFarInFuture)?;
			ensure!(last_slot <= last_slot_limit, Error::<T>::LastSlotTooFarInFuture);

			let deposit = T::SubmissionDeposit::get();

			T::MultiCurrency::reserve(T::DepositToken::get(), &depositor, deposit)?;

			Funds::<T>::insert(
				index,
				Some(FundInfo {
					depositor,
					deposit,
					raised: Zero::zero(),
					cap,
					first_slot,
					last_slot,
					trie_index: Self::next_trie_index()?,
					status: FundStatus::Ongoing,
				}),
			);

			Self::deposit_event(Event::<T>::Created(index));

			Ok(())
		}

		/// TODO: Refactor the docs.
		/// Contribute to a crowd sale. This will transfer some balance over to fund a parachain
		/// slot. It will be withdrawable in two instances: the parachain becomes retired; or the
		/// slot is unable to be purchased and the timeout expires.
		#[pallet::weight(T::WeightInfo::contribute())]
		pub fn contribute(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::Ongoing, Error::<T>::InvalidFundStatus);

			ensure!(value >= T::MinContribution::get(), Error::<T>::ContributionTooSmall);

			let raised = fund.raised.checked_add(&value).ok_or(Error::<T>::Overflow)?;
			ensure!(raised <= fund.cap, Error::<T>::CapExceeded);

			let (contributed, status) = Self::contribution(fund.trie_index, &who);
			ensure!(status == ContributionStatus::Idle, Error::<T>::InvalidContributionStatus);

			Self::xcm_ump_contribute(origin, index, value).map_err(|_e| Error::<T>::XcmFailed)?;

			Self::put_contribution(
				fund.trie_index,
				&who,
				contributed,
				ContributionStatus::Contributing(value),
			);

			Self::deposit_event(Event::Contributing(who, index, value));

			Ok(())
		}

		/// Confirm contribute
		#[pallet::weight(0)]
		pub fn confirm_contribute(
			origin: OriginFor<T>,
			who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
			is_success: bool,
		) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			let can_confirm = fund.status == FundStatus::Ongoing ||
				fund.status == FundStatus::Failed ||
				fund.status == FundStatus::Success;
			ensure!(can_confirm, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			let (contributed, status) = Self::contribution(fund.trie_index, &who);
			ensure!(status.is_contributing(), Error::<T>::InvalidContributionStatus);
			let contributing = status.contributing();

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);

			if is_success {
				// Issue reserved vsToken/vsBond to contributor
				T::MultiCurrency::deposit(vsToken, &who, contributing)?;
				T::MultiCurrency::reserve(vsToken, &who, contributing)?;
				T::MultiCurrency::deposit(vsBond, &who, contributing)?;
				T::MultiCurrency::reserve(vsBond, &who, contributing)?;

				// Update the raised of fund
				let fund_new =
					FundInfo { raised: fund.raised.saturating_add(contributing), ..fund };
				Funds::<T>::insert(index, Some(fund_new));

				// Update the contribution of who
				let contributed_new = contributed.saturating_add(contributing);
				Self::put_contribution(
					fund.trie_index,
					&who,
					contributed_new,
					ContributionStatus::Idle,
				);

				Self::deposit_event(Event::Contributed(who, index, contributing));
			} else {
				// Update the contribution of who
				Self::put_contribution(
					fund.trie_index,
					&who,
					contributed,
					ContributionStatus::Idle,
				);

				Self::deposit_event(Event::ContributeFailed(who, index, contributing));
			}

			Ok(())
		}

		/// Withdraw full balance of the parachain. this function may need to be called multiple
		/// times
		/// - `index`: The parachain to whose crowdloan the contribution was made.
		#[pallet::weight(0)]
		pub fn withdraw(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			let depositor = ensure_signed(origin.clone())?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			let can = fund.status == FundStatus::Failed || fund.status == FundStatus::Retired;
			ensure!(can, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			Self::xcm_ump_withdraw(origin, index).map_err(|_| Error::<T>::XcmFailed)?;

			Self::deposit_event(Event::Withdrawing(depositor, index, fund.raised));

			Ok(())
		}

		/// Confirm withdraw by fund owner temporarily
		#[pallet::weight(0)]
		pub fn confirm_withdraw(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			is_success: bool,
		) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			let can = fund.status == FundStatus::Failed || fund.status == FundStatus::Retired;
			ensure!(can, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			let amount_withdrew = fund.raised;

			if is_success {
				if fund.status == FundStatus::Retired {
					RedeemPool::<T>::set(Self::redeem_pool().saturating_add(amount_withdrew));

					let fund_new = FundInfo { status: FundStatus::RedeemWithdrew, ..fund };
					Funds::<T>::insert(index, Some(fund_new));
				} else if fund.status == FundStatus::Failed {
					RefundPool::<T>::set(Self::refund_pool().saturating_add(amount_withdrew));

					let fund_new = FundInfo { status: FundStatus::RefundWithdrew, ..fund };
					Funds::<T>::insert(index, Some(fund_new));
				}

				Self::deposit_event(Event::Withdrew(depositor, index, amount_withdrew));
			} else {
				Self::deposit_event(Event::WithdrawFailed(depositor, index, amount_withdrew));
			}

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn refund(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::RefundWithdrew, Error::<T>::InvalidFundStatus);

			let (contributed, status) = Self::contribution(fund.trie_index, &who);
			ensure!(contributed > Zero::zero(), Error::<T>::ZeroContribution);
			ensure!(status == ContributionStatus::Idle, Error::<T>::InvalidContributionStatus);

			ensure!(Self::refund_pool() >= contributed, Error::<T>::NotEnoughBalanceInRefundPool);

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);
			ensure!(
				T::MultiCurrency::reserved_balance(vsToken, &who) >= contributed,
				Error::<T>::NotEnoughReservedAssetsToRefund
			);
			ensure!(
				T::MultiCurrency::reserved_balance(vsBond, &who) >= contributed,
				Error::<T>::NotEnoughReservedAssetsToRefund
			);

			Self::xcm_ump_transfer(origin, index, contributed)
				.map_err(|_| Error::<T>::XcmFailed)?;

			RefundPool::<T>::set(Self::refund_pool().saturating_sub(contributed));

			Self::put_contribution(
				fund.trie_index,
				&who,
				contributed,
				ContributionStatus::Refunding,
			);

			Self::deposit_event(Event::Refunding(who, index, contributed));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn confirm_refund(
			origin: OriginFor<T>,
			who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
			is_success: bool,
		) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			let fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::RefundWithdrew, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			let (contributed, status) = Self::contribution(fund.trie_index, &who);
			ensure!(status == ContributionStatus::Refunding, Error::<T>::InvalidContributionStatus);

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, fund.first_slot, fund.last_slot);

			if is_success {
				// Slash the reserved vsToken/vsBond
				let balance = T::MultiCurrency::slash_reserved(vsToken, &who, contributed);
				ensure!(balance == Zero::zero(), Error::<T>::NotEnoughReservedAssetsToRefund);
				let balance = T::MultiCurrency::slash_reserved(vsBond, &who, contributed);
				ensure!(balance == Zero::zero(), Error::<T>::NotEnoughReservedAssetsToRefund);

				Self::put_contribution(
					fund.trie_index,
					&who,
					contributed,
					ContributionStatus::Refunded,
				);

				Self::deposit_event(Event::Refunded(who, index, contributed));
			} else {
				RefundPool::<T>::set(Self::refund_pool().saturating_add(contributed));

				Self::put_contribution(
					fund.trie_index,
					&who,
					contributed,
					ContributionStatus::Idle,
				);

				Self::deposit_event(Event::RefundFailed(who, index, contributed));
			}

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn redeem(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] first_slot: LeasePeriod,
			#[pallet::compact] last_slot: LeasePeriod,
			#[pallet::compact] value: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;

			ensure!(Self::redeem_pool() >= value, Error::<T>::NotEnoughBalanceInRedeemPool);

			let cur_block = <frame_system::Pallet<T>>::block_number();
			ensure!(!Self::is_expired(cur_block, last_slot), Error::<T>::VSBondExpired);
			ensure!(Self::can_redeem(cur_block, last_slot), Error::<T>::UnRedeemableNow);

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, first_slot, last_slot);

			T::MultiCurrency::reserve(vsToken, &who, value)
				.map_err(|_| Error::<T>::NotEnoughFreeAssetsToRedeem)?;
			T::MultiCurrency::reserve(vsBond, &who, value)
				.map_err(|_| Error::<T>::NotEnoughFreeAssetsToRedeem)?;

			let status = Self::redeem_status(who.clone(), (index, first_slot, last_slot));
			ensure!(status == RedeemStatus::Idle, Error::<T>::InvalidRedeemStatus);

			Self::xcm_ump_transfer(origin.clone(), index, value)
				.map_err(|_| Error::<T>::XcmFailed)?;

			RedeemPool::<T>::set(Self::redeem_pool().saturating_sub(value));

			RedeemExtras::<T>::insert(
				who.clone(),
				(index, first_slot, last_slot),
				RedeemStatus::Redeeming(value),
			);

			Self::deposit_event(Event::Redeeming(who, index, first_slot, last_slot, value));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn confirm_redeem(
			origin: OriginFor<T>,
			who: AccountIdOf<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] first_slot: LeasePeriod,
			#[pallet::compact] last_slot: LeasePeriod,
			is_success: bool,
		) -> DispatchResult {
			use RedeemStatus as RS;

			ensure_root(origin).map_err(|_| Error::<T>::UnauthorizedAccount)?;

			let status = Self::redeem_status(who.clone(), (index, first_slot, last_slot));
			ensure!(status.is_redeeming(), Error::<T>::InvalidRedeemStatus);
			let value = status.redeeming();

			#[allow(non_snake_case)]
			let (vsToken, vsBond) = Self::vsAssets(index, first_slot, last_slot);

			if is_success {
				let balance = T::MultiCurrency::slash_reserved(vsToken, &who, value);
				ensure!(balance == Zero::zero(), Error::<T>::NotEnoughFreeAssetsToRedeem);
				let balance = T::MultiCurrency::slash_reserved(vsBond, &who, value);
				ensure!(balance == Zero::zero(), Error::<T>::NotEnoughFreeAssetsToRedeem);

				RedeemExtras::<T>::insert(who.clone(), (index, first_slot, last_slot), RS::Idle);

				Self::deposit_event(Event::Redeemed(who, index, first_slot, last_slot, value));
			} else {
				let balance = T::MultiCurrency::unreserve(vsToken, &who, value);
				ensure!(
					balance == Zero::zero(),
					Error::<T>::NotEnoughReservedAssetsToUnlockWhenRedeemFailed
				);
				let balance = T::MultiCurrency::unreserve(vsBond, &who, value);
				ensure!(
					balance == Zero::zero(),
					Error::<T>::NotEnoughReservedAssetsToUnlockWhenRedeemFailed
				);

				RedeemPool::<T>::set(Self::redeem_pool().saturating_add(value));

				RedeemExtras::<T>::insert(who.clone(), (index, first_slot, last_slot), RS::Idle);

				Self::deposit_event(Event::RedeemFailed(who, index, first_slot, last_slot, value));
			}

			Ok(())
		}

		/// Remove a fund after the retirement period has ended and all funds have been returned.
		#[pallet::weight(0)]
		pub fn dissolve(origin: OriginFor<T>, #[pallet::compact] index: ParaId) -> DispatchResult {
			let depositor = ensure_signed(origin)?;

			let mut fund = Self::funds(index).ok_or(Error::<T>::InvalidParaId)?;
			ensure!(fund.status == FundStatus::End, Error::<T>::InvalidFundStatus);

			ensure!(depositor == fund.depositor, Error::<T>::UnauthorizedAccount);

			// TODO: Delete element when iter? Fix it?
			let mut refund_count = 0u32;
			// Try killing the crowdloan child trie and Assume everyone will be refunded.
			let contributions = Self::contribution_iterator(fund.trie_index);
			let mut all_refunded = true;
			for (who, (balance, _)) in contributions {
				if refund_count >= T::RemoveKeysLimit::get() {
					// Not everyone was able to be refunded this time around.
					all_refunded = false;
					break;
				}
				Self::kill_contribution(fund.trie_index, &who);
				fund.raised = fund.raised.saturating_sub(balance);
				refund_count += 1;
			}

			if all_refunded == true {
				T::MultiCurrency::unreserve(T::DepositToken::get(), &depositor, fund.deposit);
				Funds::<T>::remove(index);
				Self::deposit_event(Event::<T>::Dissolved(index));
			}

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			// Release x% KSM/DOT from redeem-pool to bancor-pool per cycle
			if n != 0 && (n % T::ReleaseCycle::get()) == 0 {
				if let Ok(rp_balance) = TryInto::<u128>::try_into(Self::redeem_pool()) {
					// Calculate the release amount
					let release_amount = T::ReleaseRatio::get() * rp_balance;

					// Must be ok
					if let Ok(release_amount) = TryInto::<BalanceOf<T>>::try_into(release_amount) {
						RedeemPool::<T>::set(Self::redeem_pool().saturating_sub(release_amount));

						// Increase the balance of bancor-pool by release-amount
						if let Err(err) =
							T::BancorPool::add_token(T::RelayChainToken::get(), release_amount)
						{
							log::warn!("Bancor: {:?} on bifrost-bancor.", err);
						}
					} else {
						log::warn!("Overflow: The balance of redeem-pool exceeds u128.");
					}
				}
			}

			// TODO: Auto unlock vsToken/vsBond?
		}

		fn on_initialize(_n: BlockNumberFor<T>) -> frame_support::weights::Weight {
			// TODO estimate weight
			Zero::zero()
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn id_from_index(index: TrieIndex) -> child::ChildInfo {
			let mut buf = Vec::new();
			buf.extend_from_slice(&(T::PalletId::get().0));
			buf.extend_from_slice(&index.encode()[..]);
			child::ChildInfo::new_default(T::Hashing::hash(&buf[..]).as_ref())
		}

		pub(crate) fn contribution(
			index: TrieIndex,
			who: &AccountIdOf<T>,
		) -> (BalanceOf<T>, ContributionStatus<BalanceOf<T>>) {
			who.using_encoded(|b| {
				child::get_or_default::<(BalanceOf<T>, ContributionStatus<BalanceOf<T>>)>(
					&Self::id_from_index(index),
					b,
				)
			})
		}

		pub(crate) fn contribution_iterator(
			index: TrieIndex,
		) -> ChildTriePrefixIterator<(
			AccountIdOf<T>,
			(BalanceOf<T>, ContributionStatus<BalanceOf<T>>),
		)> {
			ChildTriePrefixIterator::<_>::with_prefix_over_key::<Identity>(
				&Self::id_from_index(index),
				&[],
			)
		}

		pub(crate) fn next_trie_index() -> Result<TrieIndex, Error<T>> {
			CurrentTrieIndex::<T>::try_mutate(|ti| {
				*ti = ti.checked_add(1).ok_or(Error::<T>::Overflow)?;
				Ok(*ti - 1)
			})
		}

		#[allow(non_snake_case)]
		pub(crate) fn vsAssets(
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
		) -> (CurrencyId, CurrencyId) {
			let token_symbol = *T::RelayChainToken::get();

			let vsToken = CurrencyId::VSToken(token_symbol);
			let vsBond = CurrencyId::VSBond(token_symbol, index, first_slot, last_slot);

			(vsToken, vsBond)
		}

		/// Check if the vsBond is `past` the redeemable date
		pub(crate) fn is_expired(block: BlockNumberFor<T>, last_slot: LeasePeriod) -> bool {
			let block_begin_redeem = Self::block_end_of_lease_period_index(last_slot);
			let block_end_redeem = block_begin_redeem + T::VSBondValidPeriod::get();

			block >= block_end_redeem
		}

		/// Check if the vsBond is `in` the redeemable date
		pub(crate) fn can_redeem(block: BlockNumberFor<T>, last_slot: LeasePeriod) -> bool {
			let block_begin_redeem = Self::block_end_of_lease_period_index(last_slot);
			let block_end_redeem = block_begin_redeem + T::VSBondValidPeriod::get();

			block >= block_begin_redeem && block < block_end_redeem
		}

		#[allow(unused)]
		pub(crate) fn block_start_of_lease_period_index(slot: LeasePeriod) -> BlockNumberFor<T> {
			slot * T::LeasePeriod::get()
		}

		pub(crate) fn block_end_of_lease_period_index(slot: LeasePeriod) -> BlockNumberFor<T> {
			(slot + 1) * T::LeasePeriod::get()
		}

		fn put_contribution(
			index: TrieIndex,
			who: &AccountIdOf<T>,
			contributed: BalanceOf<T>,
			status: ContributionStatus<BalanceOf<T>>,
		) {
			who.using_encoded(|b| {
				child::put(&Self::id_from_index(index), b, &(contributed, status))
			});
		}

		fn kill_contribution(index: TrieIndex, who: &AccountIdOf<T>) {
			who.using_encoded(|b| child::kill(&Self::id_from_index(index), b));
		}

		fn xcm_ump_contribute(
			origin: OriginFor<T>,
			index: ParaId,
			value: BalanceOf<T>,
		) -> XcmResult {
			let origin_location: MultiLocation =
				T::ExecuteXcmOrigin::ensure_origin(origin).map_err(|_e| XcmError::BadOrigin)?;

			let contribution = Contribution { index, value: value.clone(), signature: None };

			let call = CrowdloanContributeCall::CrowdloanContribute(ContributeCall::Contribute(
				contribution,
			))
			.encode()
			.into();

			let amount = TryInto::<u128>::try_into(value).map_err(|_| XcmError::Unimplemented)?;

			let _result = T::BifrostXcmExecutor::ump_transfer_asset(
				origin_location.clone(),
				MultiLocation::X1(Junction::Parachain(index)),
				amount,
				true,
			)?;

			T::BifrostXcmExecutor::ump_transact(origin_location, call)
		}

		fn xcm_ump_withdraw(origin: OriginFor<T>, index: ParaId) -> XcmResult {
			let origin_location: MultiLocation =
				T::ExecuteXcmOrigin::ensure_origin(origin).map_err(|_e| XcmError::BadOrigin)?;

			let who: AccountIdOf<T> = PolkadotParaId::from(index).into_account();

			let withdraw = Withdraw { who, index };
			let call = CrowdloanWithdrawCall::CrowdloanWithdraw(WithdrawCall::Withdraw(withdraw))
				.encode()
				.into();
			T::BifrostXcmExecutor::ump_transact(origin_location, call)
		}

		fn xcm_ump_transfer(origin: OriginFor<T>, index: ParaId, value: BalanceOf<T>) -> XcmResult {
			let origin_location: MultiLocation =
				T::ExecuteXcmOrigin::ensure_origin(origin).map_err(|_e| XcmError::BadOrigin)?;

			let amount = TryInto::<u128>::try_into(value).map_err(|_| XcmError::Unimplemented)?;

			T::BifrostXcmExecutor::ump_transfer_asset(
				MultiLocation::X1(Junction::Parachain(index)),
				origin_location,
				amount,
				false,
			)
		}
	}
}
