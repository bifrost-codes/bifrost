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

#![cfg(test)]

use frame_support::{assert_noop, assert_ok};

use crate::{mock::*, *};

#[test]
fn exchange_for_token_should_work() {
	ExtBuilder::default().one_thousand_for_alice_n_bob().build().execute_with(|| {
		// Check if the bancor pools have already been initialized.
		let ksm_pool = Bancor::get_bancor_pool(KSM).unwrap();
		let dot_pool = Bancor::get_bancor_pool(DOT).unwrap();

		assert_eq!(
			ksm_pool,
			BancorPool {
				currency_id: CurrencyId::Token(TokenSymbol::KSM),
				token_pool: 0,
				vstoken_pool: 0,
				token_ceiling: 0,
				token_base_supply: 2 * VSKSM_BASE_SUPPLY,
				vstoken_base_supply: VSKSM_BASE_SUPPLY
			}
		);
		assert_eq!(
			dot_pool,
			BancorPool {
				currency_id: CurrencyId::Token(TokenSymbol::DOT),
				token_pool: 0,
				vstoken_pool: 0,
				token_ceiling: 0,
				token_base_supply: 2 * VSDOT_BASE_SUPPLY,
				vstoken_base_supply: VSDOT_BASE_SUPPLY
			}
		);

		// the pool has no real DOT
		assert_noop!(
			Bancor::exchange_for_token(Origin::signed(ALICE), DOT, 50, 48),
			Error::<Test>::TokenSupplyNotEnough
		);

		let updated_pool = BancorPool {
			currency_id: dot_pool.currency_id,
			token_pool: dot_pool.token_pool,
			vstoken_pool: dot_pool.vstoken_pool,
			token_ceiling: 100,
			token_base_supply: dot_pool.token_base_supply,
			vstoken_base_supply: dot_pool.vstoken_base_supply,
		};

		// add some DOTs to the pool
		BancorPools::<Test>::insert(DOT, updated_pool);

		// exchange rate is lower than the specified slippage(49 <50), which leads to exchange
		// failure.
		assert_noop!(
			Bancor::exchange_for_token(Origin::signed(ALICE), DOT, 50, 50),
			Error::<Test>::PriceNotQualified
		);

		// exchange succeeds.
		assert_ok!(Bancor::exchange_for_token(Origin::signed(ALICE), DOT, 50, 48));
		let dot_pool = Bancor::get_bancor_pool(DOT).unwrap();
		assert_eq!(
			dot_pool,
			BancorPool {
				currency_id: CurrencyId::Token(TokenSymbol::DOT),
				token_pool: 49,
				vstoken_pool: 50,
				token_ceiling: 51,
				token_base_supply: 2 * VSDOT_BASE_SUPPLY,
				vstoken_base_supply: VSDOT_BASE_SUPPLY
			}
		);

		// check ALICE's account balances. ALICE should have 1000-50 VSDOT, and 1000+49 DOT.
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 1049);
		assert_eq!(Tokens::free_balance(VSDOT, &ALICE), 950);
	});
}

#[test]
fn exchange_for_vstoken_should_work() {
	ExtBuilder::default().one_thousand_for_alice_n_bob().build().execute_with(|| {
		// Check if the bancor pools have already been initialized.
		let ksm_pool = Bancor::get_bancor_pool(KSM).unwrap();
		let dot_pool = Bancor::get_bancor_pool(DOT).unwrap();

		assert_eq!(
			ksm_pool,
			BancorPool {
				currency_id: CurrencyId::Token(TokenSymbol::KSM),
				token_pool: 0,
				vstoken_pool: 0,
				token_ceiling: 0,
				token_base_supply: 2 * VSKSM_BASE_SUPPLY,
				vstoken_base_supply: VSKSM_BASE_SUPPLY
			}
		);
		assert_eq!(
			dot_pool,
			BancorPool {
				currency_id: CurrencyId::Token(TokenSymbol::DOT),
				token_pool: 0,
				vstoken_pool: 0,
				token_ceiling: 0,
				token_base_supply: 2 * VSDOT_BASE_SUPPLY,
				vstoken_base_supply: VSDOT_BASE_SUPPLY
			}
		);

		// the pool has no real VSDOT
		assert_noop!(
			Bancor::exchange_for_vstoken(Origin::signed(ALICE), DOT, 50, 48),
			Error::<Test>::VSTokenSupplyNotEnough
		);

		let updated_pool = BancorPool {
			currency_id: dot_pool.currency_id,
			token_pool: 50,
			vstoken_pool: 50,
			token_ceiling: 0,
			token_base_supply: dot_pool.token_base_supply,
			vstoken_base_supply: dot_pool.vstoken_base_supply,
		};

		// add some VSDOTs to the pool
		BancorPools::<Test>::insert(DOT, updated_pool);
		let dot_pool = Bancor::get_bancor_pool(DOT).unwrap();
		assert_eq!(
			dot_pool,
			BancorPool {
				currency_id: CurrencyId::Token(TokenSymbol::DOT),
				token_pool: 50,
				vstoken_pool: 50,
				token_ceiling: 0,
				token_base_supply: 2 * VSDOT_BASE_SUPPLY,
				vstoken_base_supply: VSDOT_BASE_SUPPLY
			}
		);

		// exchange rate is lower than the specified slippage(49 <50), which leads to exchange
		// failure.
		assert_noop!(
			Bancor::exchange_for_vstoken(Origin::signed(ALICE), DOT, 49, 50),
			Error::<Test>::PriceNotQualified
		);

		// exchange succeeds.
		assert_ok!(Bancor::exchange_for_vstoken(Origin::signed(ALICE), DOT, 49, 48));
		let dot_pool = Bancor::get_bancor_pool(DOT).unwrap();
		assert_eq!(
			dot_pool,
			BancorPool {
				currency_id: CurrencyId::Token(TokenSymbol::DOT),
				token_pool: 1,
				vstoken_pool: 1,
				token_ceiling: 0,
				token_base_supply: 2 * VSDOT_BASE_SUPPLY,
				vstoken_base_supply: VSDOT_BASE_SUPPLY
			}
		);

		// check ALICE's account balances. ALICE should have 1000-49 DOT, and 1000+49 VSDOT.
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 951);
		assert_eq!(Tokens::free_balance(VSDOT, &ALICE), 1049);
	});
}

#[test]
fn add_token_should_work() {
	ExtBuilder::default()
		.hundred_thousand_for_alice_n_bob()
		.build()
		.execute_with(|| {
			// At the beginning, the price is 1:1, so all the released token should be put into
			// ceiling.
			assert_ok!(Bancor::add_token(DOT, 20000));

			let dot_pool = Bancor::get_bancor_pool(DOT).unwrap();
			assert_eq!(
				dot_pool,
				BancorPool {
					currency_id: CurrencyId::Token(TokenSymbol::DOT),
					token_pool: 0,
					vstoken_pool: 0,
					token_ceiling: 20000,
					token_base_supply: 2 * VSDOT_BASE_SUPPLY,
					vstoken_base_supply: VSDOT_BASE_SUPPLY
				}
			);

			// if someone buys a lot of tokens, the price of token will dramatically increase and
			// the price of vstoken will decrease. Here 20_000 vsDOT can only exchange for 14_641
			// DOT. So the price of vstoken is 73.205% of token. If currently some tokens are
			// release, half of them will be put in the ceiling variable while the others will used
			// to buy back vstokens.
			let price = Bancor::calculate_price_for_token(DOT, 20000).unwrap();
			assert_ok!(Bancor::exchange_for_token(Origin::signed(ALICE), DOT, 20000, 1));
			let dot_pool = Bancor::get_bancor_pool(DOT).unwrap();
			assert_eq!(
				dot_pool,
				BancorPool {
					currency_id: CurrencyId::Token(TokenSymbol::DOT),
					token_pool: price,
					vstoken_pool: 20000,
					token_ceiling: 20000 - price,
					token_base_supply: 2 * VSDOT_BASE_SUPPLY,
					vstoken_base_supply: VSDOT_BASE_SUPPLY
				}
			);

			// token_ceiling should be 5359 + 50 = 5409, token_pool should be 14641 - 50 = 14591
			let price = Bancor::calculate_price_for_vstoken(DOT, 50).unwrap();
			assert_ok!(Bancor::add_token(DOT, 100));
			let dot_pool = Bancor::get_bancor_pool(DOT).unwrap();
			assert_eq!(
				dot_pool,
				BancorPool {
					currency_id: CurrencyId::Token(TokenSymbol::DOT),
					token_pool: 14641 - 50,
					vstoken_pool: 20000 - price,
					token_ceiling: 5359 + 50,
					token_base_supply: 2 * VSDOT_BASE_SUPPLY,
					vstoken_base_supply: VSDOT_BASE_SUPPLY
				}
			);
		});
}
