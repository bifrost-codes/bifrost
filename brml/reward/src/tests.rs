// Copyright 2019-2020 Liebi Technologies.
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

//! Tests for the module.
#![cfg(test)]

use crate::*;
use crate::mock::*;
use node_primitives::{TokenSymbol, RewardTrait};

#[test]
fn record_reward_should_be_ok() {
	new_test_ext().execute_with(|| {
		common();
	});
}

pub fn common() {
	// Ready data
	let vdot_id = assets::Module::<Test>::next_asset_id();
	let vtoken_symbol = TokenSymbol::from(vdot_id + 2);
	assert_eq!(vtoken_symbol, TokenSymbol::vDOT);
	let convert_amount = 100 as u64;
	let (referer_one, referer_two) = (168 as u64, 100 as u64);
	
	// Add new referer
	if let Err(e) = <crate::Module<Test>>::record_reward(vtoken_symbol, convert_amount, referer_one) {
		dbg!(" ====== record_reward Execute Err : \n ", e);
	};
	assert_eq!(1, crate::Module::<Test>::vtoken_reward(vtoken_symbol).len());
	dbg!("=====One vDot: 1 referer=====",<crate::Module<Test>>::vtoken_reward(vtoken_symbol));
	
	// Increate different referer
	if let Err(e) = <crate::Module<Test>>::record_reward(vtoken_symbol, convert_amount, referer_two) {
		dbg!(" ====== record_reward Execute Err : \n ", e);
	};
	assert_eq!(2, crate::Reward::<Test>::get(vtoken_symbol).len());
	dbg!("=====Two vDot: 2 referer=====", <crate::Module<Test>>::vtoken_reward(vtoken_symbol));
	
	// Append exist referer
	if let Err(e) = <crate::Module<Test>>::record_reward(vtoken_symbol, convert_amount, referer_one) {
		dbg!(" ====== record_reward Execute Err : \n ", e);
	};
	
	assert_eq!(2, <crate::Reward<Test>>::get(vtoken_symbol).len());
	dbg!("=====Three vDot: 2 referer=====", <crate::Module<Test>>::vtoken_reward(vtoken_symbol));
	
	// // Different vtoken （another table）
	let vtoken_symbol = TokenSymbol::vEOS;
	if let Err(e) = <crate::Module<Test>>::record_reward(vtoken_symbol, convert_amount, referer_one) {
		dbg!(" ====== record_reward Execute Err : \n ", e);
	};
	assert_eq!(1, <crate::Module<Test>>::vtoken_reward(vtoken_symbol).len());
	dbg!("======vEos 1 referer======\n ", <crate::Module<Test>>::vtoken_reward(vtoken_symbol));
}

#[test]
fn dispatch_reward_is_be_ok() {
	new_test_ext().execute_with(|| {
		// Condition initial
		common();
		let (vtoken_symbol,
			referer_one,
			referer_two,
			staking_profit
		) = (TokenSymbol::vDOT, 168 as u64, 100 as u64, 60 as u64);
		
		// The first query asset
		let referer_one_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_one));
		assert_eq!(0, referer_one_assets.balance);
		let referer_two_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_two));
		assert_eq!(0, referer_two_assets.balance);
		
		// Dispatch reward:
		if let Err(e) = crate::Module::<Test>::dispatch_reward(vtoken_symbol, staking_profit) {
			dbg!(" ====== dispatch_reward Execute Err : \n ", e);
		};
		
		// The second query asset
		let referer_one_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_one));
		debug_assert_eq!(40, referer_one_assets.balance);
		let referer_two_assets = assets::Module::<Test>::account_assets((vtoken_symbol, referer_two));
		debug_assert_eq!(20, referer_two_assets.balance);
		
		// Judge vtoken table whether be clear
		debug_assert!(<crate::Module<Test>>::vtoken_reward(vtoken_symbol).is_empty());
	});
}
