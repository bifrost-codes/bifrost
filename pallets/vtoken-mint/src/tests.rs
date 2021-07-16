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

//! Tests for the module.
#![cfg(test)]

use frame_support::{assert_noop, assert_ok};
use node_primitives::Balance;

use crate::{mock::*, *};

#[test]
fn mint_vtoken_should_be_ok() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let (dot_pool, vdot_pool) = (100, 200);
		assert_ok!(VtokenMint::expand_mint_pool(DOT, dot_pool));
		assert_ok!(VtokenMint::expand_mint_pool(vDOT, vdot_pool));

		let alice_dot = Tokens::free_balance(DOT, &ALICE);
		let alice_vdot = Tokens::free_balance(vDOT, &ALICE);

		let dot_price = vdot_pool / dot_pool;
		let to_sell_dot = 20;
		let minted_vdot = to_sell_dot * dot_price;

		run_to_block(1);

		// Alice sell 20 DOTs to mint vDOT.
		assert_ok!(VtokenMint::mint(Origin::signed(ALICE), vDOT, to_sell_dot));
		run_to_block(2);
		println!("MInter: {:?}", MinterReward::current_round_start_at());

		// Check event
		let mint_vtoken_event =
			mock::Event::VtokenMint(crate::Event::Minted(ALICE, vDOT, minted_vdot));
		assert!(System::events().iter().any(|record| record.event == mint_vtoken_event));

		// check Alice DOTs and vDOTs.
		assert_eq!(Tokens::free_balance(DOT, &ALICE), alice_dot - to_sell_dot);
		assert_eq!(Tokens::free_balance(vDOT, &ALICE), alice_vdot + minted_vdot);

		// check total DOTs and vDOTs.
		assert_eq!(VtokenMint::get_mint_pool(DOT), dot_pool + to_sell_dot);
		assert_eq!(VtokenMint::get_mint_pool(vDOT), vdot_pool + minted_vdot);

		// Alice selling BNC should not work.
		assert_noop!(
			VtokenMint::mint(Origin::signed(ALICE), BNC, to_sell_dot),
			Error::<Runtime>::NotSupportTokenType
		);

		// Alice selling DOT should not work due to it only support minting vDOT(vtoken).
		assert_noop!(
			VtokenMint::mint(Origin::signed(ALICE), DOT, to_sell_dot),
			Error::<Runtime>::NotSupportTokenType
		);

		// Alice selling 0 DOTs should not work.
		assert_noop!(
			VtokenMint::mint(Origin::signed(ALICE), vDOT, 0),
			Error::<Runtime>::BalanceZero
		);

		// Alice selling amount of DOTs exceeds all she has.
		assert_noop!(
			VtokenMint::mint(Origin::signed(ALICE), vDOT, Balance::max_value()),
			Error::<Runtime>::BalanceLow
		);
	});
}

#[test]
fn redeem_token_should_be_ok() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let (dot_pool, vdot_pool) = (100, 200);
		assert_ok!(VtokenMint::expand_mint_pool(DOT, dot_pool));
		assert_ok!(VtokenMint::expand_mint_pool(vDOT, vdot_pool));

		let alice_dot = Tokens::free_balance(DOT, &ALICE);
		let alice_vdot = Tokens::free_balance(vDOT, &ALICE);

		let to_sell_vdot = 20;
		let minted_dot = to_sell_vdot * dot_pool / vdot_pool;

		run_to_block(1);
		// Alice sell 20 vDOTs to mint DOT.
		assert_ok!(VtokenMint::redeem(Origin::signed(ALICE), DOT, to_sell_vdot));

		// Check event
		let redeem_token_event =
			mock::Event::VtokenMint(crate::Event::RedeemStarted(ALICE, vDOT, to_sell_vdot, 1));
		assert!(System::events().iter().any(|record| { record.event == redeem_token_event }));

		// check Alice DOTs and vDOTs.
		// assert_eq!(Tokens::free_balance(DOT, &ALICE), alice_dot + minted_dot);
		assert_eq!(Tokens::free_balance(vDOT, &ALICE), alice_vdot - to_sell_vdot);

		// check total DOTs and vDOTs.
		assert_eq!(VtokenMint::get_mint_pool(DOT), dot_pool - minted_dot);
		assert_eq!(VtokenMint::get_mint_pool(vDOT), vdot_pool - to_sell_vdot);

		// Alice selling AUSD should not work.
		assert_noop!(
			VtokenMint::redeem(Origin::signed(ALICE), AUSD, to_sell_vdot),
			Error::<Runtime>::NotSupportTokenType
		);

		// Alice selling vDOT should not work due to it only support minting DOT(token).
		assert_noop!(
			VtokenMint::redeem(Origin::signed(ALICE), vDOT, to_sell_vdot),
			Error::<Runtime>::NotSupportTokenType
		);

		// Alice selling 0 vDOTs should not work.
		assert_noop!(
			VtokenMint::redeem(Origin::signed(ALICE), DOT, 0),
			Error::<Runtime>::BalanceZero
		);

		// Alice selling amount of DOTs exceeds all she has.
		assert_noop!(
			VtokenMint::redeem(Origin::signed(ALICE), DOT, Balance::max_value()),
			Error::<Runtime>::BalanceLow
		);

		run_to_block(20);

		// Alice should have not received the minted dots, since dot redeem period is 28 blocks
		// which is set in the mock
		assert_eq!(Tokens::free_balance(DOT, &ALICE), alice_dot);

		// After 30 blocks, Alice should received the minted dots
		run_to_block(30);
		assert_eq!(Tokens::free_balance(DOT, &ALICE), alice_dot + minted_dot);
	});
}

#[test]
fn zero_token_pool_should_not_work() {
	ExtBuilder::default().zero_for_alice_n_bob().build().execute_with(|| {
		System::set_block_number(1);

		let to_sell_vdot = 20;
		let to_sell_ksm = 20;

		// Alice sell 20 vDOTs to mint DOT.
		assert_noop!(
			VtokenMint::redeem(Origin::signed(ALICE), DOT, to_sell_vdot),
			Error::<Runtime>::EmptyVtokenPool
		);

		// Alice sell 20 KSMs to mint vKSM.
		assert_noop!(
			VtokenMint::redeem(Origin::signed(BOB), vKSM, to_sell_ksm),
			Error::<Runtime>::NotSupportTokenType
		);
	});
}
