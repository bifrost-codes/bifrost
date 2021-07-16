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

//! Test utilities

#![cfg(test)]
#![allow(non_upper_case_globals)]

use core::marker::PhantomData;
use std::convert::TryInto;

use frame_support::{parameter_types, traits::GenesisBuild, PalletId};
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, UniqueSaturatedInto},
	AccountId32, DispatchError, DispatchResult, SaturatedConversion,
};
use zenlink_protocol::{AssetBalance, AssetId, LocalAssetHandler, ZenlinkMultiAssets};

use crate::{self as bifrost_minter_reward};

pub type AccountId = AccountId32;
pub const BNC: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
pub const AUSD: CurrencyId = CurrencyId::Stable(TokenSymbol::AUSD);
pub const DOT: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
pub const vDOT: CurrencyId = CurrencyId::VToken(TokenSymbol::DOT);
pub const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
pub const vKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
pub const ALICE: AccountId = AccountId32::new([0u8; 32]);
pub const BOB: AccountId = AccountId32::new([1u8; 32]);
// pub const CENTS: Balance = 1_000_000_000_000 / 100;

pub type BlockNumber = u64;
pub type Amount = i128;

pub type Balance = u64;
pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, u64, Call, ()>;

frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Storage, Event<T>},
		Assets: orml_tokens::{Pallet, Call, Storage, Event<T>, Config<T>},
		PalletBalances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		MinterReward: bifrost_minter_reward::{Pallet, Call, Storage, Event<T>, Config<T>},
		ZenlinkProtocol: zenlink_protocol::{Pallet, Call, Storage, Event<T>},
		VtokenMint: bifrost_vtoken_mint::{Pallet, Call, Storage, Event<T>},
	}
);

// type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
// type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
	pub const StableCurrencyId: CurrencyId = CurrencyId::Stable(TokenSymbol::AUSD);
}
impl frame_system::Config for Runtime {
	type AccountData = pallet_balances::AccountData<u64>;
	type AccountId = sp_runtime::AccountId32;
	type BaseCallFilter = ();
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockNumber = u64;
	type BlockWeights = ();
	type Call = Call;
	type DbWeight = ();
	type Event = Event;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Header = Header;
	type Index = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
}

pub type AdaptedBasicCurrency =
	orml_currencies::BasicCurrencyAdapter<Runtime, PalletBalances, Amount, BlockNumber>;

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Assets;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Runtime {
	type AccountStore = frame_system::Pallet<Runtime>;
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}
impl orml_tokens::Config for Runtime {
	type Amount = i128;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = ();
	type OnDust = orml_tokens::TransferDust<Runtime, ()>;
	type WeightInfo = ();
}

impl bifrost_vtoken_mint::Config for Runtime {
	type Event = Event;
	type MinterReward = MinterReward;
	type MultiCurrency = Currencies;
	type WeightInfo = ();
}

// Zenlink runtime implementation
parameter_types! {
	pub const ZenlinkPalletId: PalletId = PalletId(*b"/zenlink");
	pub const GetExchangeFee: (u32, u32) = (3, 1000);   // 0.3%
	pub const SelfParaId: u32 = 2001;
}

impl zenlink_protocol::Config for Runtime {
	type Conversion = ();
	type Event = Event;
	type GetExchangeFee = GetExchangeFee;
	type MultiAssetsHandler = MultiAssets;
	type PalletId = ZenlinkPalletId;
	// type SelfParaId = SelfParaId;
	type SelfParaId = SelfParaId;
	type TargetChains = ();
	type XcmExecutor = ();
}

type MultiAssets =
	ZenlinkMultiAssets<ZenlinkProtocol, PalletBalances, LocalAssetAdaptor<Currencies>>;

// Below is the implementation of tokens manipulation functions other than native token.
pub struct LocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local, AccountId> LocalAssetHandler<AccountId> for LocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, CurrencyId = CurrencyId>,
{
	fn local_balance_of(asset_id: AssetId, who: &AccountId) -> AssetBalance {
		let currency_id: CurrencyId = asset_id.try_into().unwrap();
		Local::free_balance(currency_id, &who).saturated_into()
	}

	fn local_total_supply(asset_id: AssetId) -> AssetBalance {
		let currency_id: CurrencyId = asset_id.try_into().unwrap();
		Local::total_issuance(currency_id).saturated_into()
	}

	fn local_is_exists(asset_id: AssetId) -> bool {
		let rs: Result<CurrencyId, _> = asset_id.try_into();
		match rs {
			Ok(_) => true,
			Err(_) => false,
		}
	}

	fn local_transfer(
		asset_id: AssetId,
		origin: &AccountId,
		target: &AccountId,
		amount: AssetBalance,
	) -> DispatchResult {
		let currency_id: CurrencyId = asset_id.try_into().unwrap();
		Local::transfer(currency_id, &origin, &target, amount.unique_saturated_into())?;

		Ok(())
	}

	fn local_deposit(
		asset_id: AssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		let currency_id: CurrencyId = asset_id.try_into().unwrap();
		Local::deposit(currency_id, &origin, amount.unique_saturated_into())?;
		return Ok(amount);
	}

	fn local_withdraw(
		asset_id: AssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		let currency_id: CurrencyId = asset_id.try_into().unwrap();
		Local::withdraw(currency_id, &origin, amount.unique_saturated_into())?;

		Ok(amount)
	}
}

// zenlink runtime ends

parameter_types! {
	pub const HalvingCycle: u32 = 60;
	pub const RewardWindow: u32 = 10;
	pub const MaximumExtendedPeriod: u32 = 20;
}

impl crate::Config for Runtime {
	type DexOperator = ZenlinkProtocol;
	type Event = Event;
	type HalvingCycle = HalvingCycle;
	type MaximumExtendedPeriod = MaximumExtendedPeriod;
	type MultiCurrency = Currencies;
	type RewardWindow = RewardWindow;
	type ShareWeight = Balance;
	type StableCurrencyId = StableCurrencyId;
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { endowed_accounts: vec![] }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn ten_thousand_for_alice_n_bob(self) -> Self {
		self.balances(vec![
			(ALICE, BNC, 100000),
			(ALICE, AUSD, 10000),
			(ALICE, vDOT, 10000),
			(ALICE, vKSM, 10000),
			(ALICE, DOT, 10000),
			(ALICE, KSM, 10000),
			(BOB, BNC, 100000),
			(BOB, AUSD, 10000),
			(BOB, vDOT, 10000),
			(BOB, vKSM, 10000),
			(BOB, DOT, 10000),
			(BOB, KSM, 10000),
		])
	}

	// pub fn one_hundred_for_alice_n_bob(self) -> Self {
	// 	self.balances(vec![
	// 		(ALICE, BNC, 100),
	// 		(BOB, BNC, 100),
	// 		(ALICE, DOT, 100),
	// 		(ALICE, vDOT, 400),
	// 		(BOB, DOT, 100),
	// 		(BOB, KSM, 100),
	// 	])
	// }

	// pub fn zero_for_alice_n_bob(self) -> Self {
	// 	self.balances(vec![
	// 		(ALICE, BNC, 100),
	// 		(BOB, BNC, 100),
	// 		(ALICE, DOT, 0),
	// 		(ALICE, vDOT, 100),
	// 		(BOB, DOT, 0),
	// 		(BOB, KSM, 100),
	// 	])
	// }

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: self
				.endowed_accounts
				.clone()
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id == BNC)
				.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> { balances: self.endowed_accounts }
			.assimilate_storage(&mut t)
			.unwrap();

		crate::GenesisConfig::<Runtime> {
			currency_weights: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 1 * 1),
				(CurrencyId::Token(TokenSymbol::ETH), 1 * 1),
				(CurrencyId::Token(TokenSymbol::KSM), 1 * 3),
			],
			reward_per_block: 300,
			cycle_index: 1,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
