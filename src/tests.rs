// Copyright (C) 2021 Georges Dib.
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

//! Unit tests for perpetualasset module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, ExtBuilder, Origin, Runtime, PerpetualAsset, System, Tokens,
	MockPriceSource,ALICE, BOB, CHARLIE, GEORGES, KUSD};

fn last_event() -> Event {
	System::events().last().unwrap().event.clone()
}

#[test]
fn top_up_collateral_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();
		PerpetualAsset::on_initialize(1);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 21i128));

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 0i128, 10i128));

		assert_eq!(PerpetualAsset::total_collateral_balance(), 31u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 30u128);

		assert_noop!(
			PerpetualAsset::mint(
				Origin::signed(ALICE),
				0i128,
				2_000_000_000_000_000_000i128,
			),
			orml_tokens::Error::<Runtime>::BalanceTooLow,
		);

		assert_eq!(PerpetualAsset::total_collateral_balance(), 31u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 30u128);
	});
}

#[test]
fn mint_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();

		assert_noop!(
			PerpetualAsset::mint(Origin::signed(ALICE), 10i128, 1i128),
			crate::Error::<Runtime>::PriceNotSet
		);

		PerpetualAsset::on_initialize(1);

		assert_noop!(
			PerpetualAsset::mint(
				Origin::signed(ALICE),
				2_000_000_000_000_000_000i128,
				2_000_000_000_000_000_000i128
			),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);
		assert_eq!(PerpetualAsset::margin(&ALICE), 0u128);

		assert_noop!(
			PerpetualAsset::mint(Origin::signed(ALICE), 10i128, 1i128),
			crate::Error::<Runtime>::NotEnoughIM
		);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 20i128));

		assert_eq!(
			last_event(),
			Event::perpetualasset(crate::Event::BalanceUpdated(ALICE, 100i128))
		);

		assert_eq!(PerpetualAsset::total_collateral_balance(), 20u128);
		assert_eq!(Tokens::total_balance(KUSD, &ALICE), 999_999_999_999_999_980u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 20u128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), -10i128, 0i128)); // Removes balance so no IM needed
		assert_eq!(PerpetualAsset::total_collateral_balance(), 20u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 20u128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 90i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 20i128, 2i128)); // Only 10 unit added, so 2 IM needed
		assert_eq!(PerpetualAsset::total_collateral_balance(), 22u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 22u128);

		// balance is now -200, so 40 IM needed, 22 already there, so need 18
		assert_noop!(
			PerpetualAsset::mint(Origin::signed(ALICE), -310i128, 17i128),
			crate::Error::<Runtime>::NotEnoughIM
		);
		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), -310i128, 18i128));
		assert_eq!(PerpetualAsset::total_collateral_balance(), 40u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 40u128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), -100i128, 20i128));
		assert_eq!(PerpetualAsset::total_collateral_balance(), 60u128);
		assert_eq!(PerpetualAsset::margin(&BOB), 20u128);
	});
}

#[test]
fn match_interest_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();

		PerpetualAsset::on_initialize(1);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 20i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), -100i128, 20i128));

		PerpetualAsset::on_initialize(2);

		assert_eq!(PerpetualAsset::inventory(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), -50i128, 0i128));
		PerpetualAsset::on_initialize(3);
		assert_eq!(PerpetualAsset::inventory(&ALICE), 50i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -50i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(CHARLIE), 100i128, 20i128));
		PerpetualAsset::on_initialize(4);
		assert_eq!(PerpetualAsset::inventory(&ALICE), 33i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 66i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(GEORGES), -100i128, 20i128));
		PerpetualAsset::on_initialize(4);
		assert_eq!(PerpetualAsset::inventory(&ALICE), 50i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 100i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -75i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -75i128);
	});
}

#[test]
fn redeem_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();
		PerpetualAsset::on_initialize(1);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 20i128));
		assert_eq!(PerpetualAsset::total_collateral_balance(), 20u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 20u128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);

		assert_noop!(
			PerpetualAsset::mint(Origin::signed(ALICE), 0i128, -1i128),
			crate::Error::<Runtime>::NotEnoughIM
		);

		assert_eq!(PerpetualAsset::total_collateral_balance(), 20u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 20u128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 60i128));
		assert_eq!(PerpetualAsset::total_collateral_balance(), 80u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 80u128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 200i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, -10i128));
		assert_eq!(PerpetualAsset::total_collateral_balance(), 70u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 70u128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 300i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 10i128));
		assert_eq!(PerpetualAsset::total_collateral_balance(), 80u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 80u128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 400i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 0i128, 10i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 10i128));

		assert_noop!(
			PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 10i128),
			crate::Error::<Runtime>::NotEnoughIM
		);
	});
}

#[test]
fn liquidate_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();
		PerpetualAsset::on_initialize(1);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 20i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), -100i128, 20i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(CHARLIE), 50i128, 20i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(GEORGES), -10i128, 20i128));
		PerpetualAsset::on_initialize(2);

		assert_eq!(PerpetualAsset::inventory(&ALICE), 73i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 36i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -10i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 50i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -10i128);

		MockPriceSource::set_price(Some(2u128.into()));
		PerpetualAsset::update_margin();

		assert_eq!(PerpetualAsset::total_collateral_balance(), 80u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 93u128);
		assert_eq!(PerpetualAsset::margin(&BOB), 0u128);
		assert_eq!(PerpetualAsset::margin(&CHARLIE), 56u128);
		assert_eq!(PerpetualAsset::margin(&GEORGES), 10u128);

		PerpetualAsset::liquidate();

		assert_eq!(PerpetualAsset::inventory(&ALICE), 73i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), 0i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 36i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -10i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::balances(&BOB), 0i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 50i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -10i128);

		PerpetualAsset::match_interest();

		assert_eq!(PerpetualAsset::inventory(&ALICE), 6i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), 0i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 3i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -10i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::balances(&BOB), 0i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 50i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -10i128);
	});
}

#[test]
fn liquidate_works_0_price() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();
		MockPriceSource::set_price(Some(20u128.into()));
		PerpetualAsset::update_margin();

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 400i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), -100i128, 400i128));
		PerpetualAsset::match_interest();

		assert_eq!(PerpetualAsset::inventory(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);

		// Price goes to 0, ALICE should be fully liquidated
		MockPriceSource::set_price(Some(0u128.into()));
		PerpetualAsset::update_margin();
		PerpetualAsset::liquidate();
		assert_eq!(PerpetualAsset::total_collateral_balance(), 800u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 0u128);
		assert_eq!(PerpetualAsset::margin(&BOB), 2400u128);

		assert_eq!(PerpetualAsset::inventory(&ALICE), 0i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 0i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
	});
}

#[test]
fn liquidate_works_complex_2() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();
		MockPriceSource::set_price(Some(20u128.into()));
		PerpetualAsset::update_margin();

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 400i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), -100i128, 400i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(CHARLIE), 100i128, 4000i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(GEORGES), 100i128, 4000i128));
		PerpetualAsset::match_interest();

		assert_eq!(PerpetualAsset::inventory(&ALICE), 33i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 33i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), 33i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 100i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), 100i128);

		// liquidate all of Alice's open interest
		MockPriceSource::set_price(Some(9u128.into()));
		PerpetualAsset::update_margin();
		PerpetualAsset::liquidate();
		assert_eq!(PerpetualAsset::total_collateral_balance(), 8800u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 37u128);
		assert_eq!(PerpetualAsset::margin(&BOB), 1500u128);
		assert_eq!(PerpetualAsset::margin(&CHARLIE), 3637u128);
		assert_eq!(PerpetualAsset::margin(&GEORGES), 3637u128);

		assert_eq!(PerpetualAsset::inventory(&ALICE), 33i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 33i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), 33i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 33i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 100i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), 100i128);

		PerpetualAsset::match_interest();
		assert_eq!(PerpetualAsset::inventory(&ALICE), 14i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 42i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), 42i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 33);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 100i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), 100i128);
	});
}

#[test]
fn liquidate_works_complex() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();
		MockPriceSource::set_price(Some(20u128.into()));
		PerpetualAsset::update_margin();

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 450i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), -100i128, 400i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(CHARLIE), 50i128, 400i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(GEORGES), -10i128, 400i128));
		PerpetualAsset::match_interest();

		assert_eq!(PerpetualAsset::inventory(&ALICE), 73i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 36i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -10i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 50i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -10i128);

		// No liquidation
		MockPriceSource::set_price(Some(19u128.into()));
		PerpetualAsset::update_margin();
		PerpetualAsset::liquidate();
		assert_eq!(PerpetualAsset::inventory(&ALICE), 73i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 36i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -10i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 50i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -10i128);

		// liquidate Alice's open interest only
		MockPriceSource::set_price(Some(16u128.into()));
		PerpetualAsset::update_margin();
		PerpetualAsset::liquidate();
		assert_eq!(PerpetualAsset::total_collateral_balance(), 1650u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 158u128);
		assert_eq!(PerpetualAsset::margin(&BOB), 800u128);
		assert_eq!(PerpetualAsset::margin(&CHARLIE), 256u128);
		assert_eq!(PerpetualAsset::margin(&GEORGES), 440u128);

		assert_eq!(PerpetualAsset::inventory(&ALICE), 73i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 36i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -10i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 73i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 50i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -10i128);

		PerpetualAsset::match_interest();
		assert_eq!(PerpetualAsset::inventory(&ALICE), 65i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 44i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -10i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 73i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 50i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -10i128);
	});
}

#[test]
fn update_balances_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();
		PerpetualAsset::on_initialize(1);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 20i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), -100i128, 20i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(CHARLIE), 50i128, 20i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(GEORGES), -10i128, 20i128));
		PerpetualAsset::on_initialize(2);

		MockPriceSource::set_price(Some(2u128.into()));
		PerpetualAsset::update_margin();

		assert_eq!(PerpetualAsset::inventory(&ALICE), 73i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 36i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -10i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 50i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -10i128);

		assert_eq!(PerpetualAsset::total_collateral_balance(), 80u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 93u128);
		assert_eq!(PerpetualAsset::margin(&BOB), 0u128);
		assert_eq!(PerpetualAsset::margin(&CHARLIE), 56u128);
		assert_eq!(PerpetualAsset::margin(&GEORGES), 10u128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), 0i128, 120i128));
		assert_eq!(PerpetualAsset::margin(&BOB), 120u128);
	});
}

#[test]
fn claim_collateral() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();
		MockPriceSource::set_price(Some(20u128.into()));
		PerpetualAsset::update_margin();

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 400i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), -100i128, 400i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(CHARLIE), 100i128, 400i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(GEORGES), -100i128, 400i128));
		PerpetualAsset::match_interest();

		assert_eq!(PerpetualAsset::inventory(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 100i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -100i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 100i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -100i128);

		// Price goes to 0, ALICE and CHARLIE should be fully liquidated
		MockPriceSource::set_price(Some(0u128.into()));
		PerpetualAsset::update_margin();
		PerpetualAsset::liquidate();
		assert_eq!(PerpetualAsset::total_collateral_balance(), 1600u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 0u128);
		assert_eq!(PerpetualAsset::margin(&BOB), 2400u128);
		assert_eq!(PerpetualAsset::margin(&CHARLIE), 0u128);
		assert_eq!(PerpetualAsset::margin(&GEORGES), 2400u128);

		assert_eq!(PerpetualAsset::inventory(&ALICE), 0i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 0i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -100i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 0i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 0i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -100i128);

		// Claim back collateral
		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), 0i128, -1600i128));
		assert_noop!(
			PerpetualAsset::mint(Origin::signed(GEORGES), 0i128, -1i128),
			orml_tokens::Error::<Runtime>::BalanceTooLow,
		);
	});
}

#[test]
fn claim_collateral_2() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();
		MockPriceSource::set_price(Some(20u128.into()));
		PerpetualAsset::update_margin();

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 400i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), -100i128, 400i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(CHARLIE), 100i128, 400i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(GEORGES), -100i128, 400i128));
		PerpetualAsset::match_interest();

		assert_eq!(PerpetualAsset::inventory(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 100i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -100i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 100i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -100i128);

		// Price goes to 0, ALICE and CHARLIE should be fully liquidated
		MockPriceSource::set_price(Some(0u128.into()));
		PerpetualAsset::update_margin();
		PerpetualAsset::liquidate();
		assert_eq!(PerpetualAsset::total_collateral_balance(), 1600u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 0u128);
		assert_eq!(PerpetualAsset::margin(&BOB), 2400u128);
		assert_eq!(PerpetualAsset::margin(&CHARLIE), 0u128);
		assert_eq!(PerpetualAsset::margin(&GEORGES), 2400u128);

		assert_eq!(PerpetualAsset::inventory(&ALICE), 0i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 0i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -100i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 0i128);
		assert_eq!(PerpetualAsset::balances(&BOB), -100i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 0i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -100i128);

		// Claim back collateral
		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), 100i128, -1600i128));
		PerpetualAsset::match_interest();
		
		MockPriceSource::set_price(Some(10u128.into()));
		PerpetualAsset::update_margin();
		PerpetualAsset::liquidate();

		assert_eq!(PerpetualAsset::total_collateral_balance(), 0u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 0u128);
		assert_eq!(PerpetualAsset::margin(&BOB), 800u128);
		assert_eq!(PerpetualAsset::margin(&CHARLIE), 0u128);
		assert_eq!(PerpetualAsset::margin(&GEORGES), 2400u128);

		assert_eq!(PerpetualAsset::inventory(&ALICE), 0i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), 0i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 0i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), 0i128);
		assert_eq!(PerpetualAsset::balances(&ALICE), 0i128);
		assert_eq!(PerpetualAsset::balances(&BOB), 0i128);
		assert_eq!(PerpetualAsset::balances(&CHARLIE), 0i128);
		assert_eq!(PerpetualAsset::balances(&GEORGES), -100i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 200i128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(GEORGES), 0i128, -200i128));
	});
}