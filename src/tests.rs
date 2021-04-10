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
	ALICE, BOB, CHARLIE, GEORGES, KUSD};

fn last_event() -> Event {
	System::events().last().unwrap().event.clone()
}

#[test]
fn top_up_collateral_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();
		PerpetualAsset::on_initialize(1);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 20u128));

		assert_ok!(PerpetualAsset::top_up_collateral(Origin::signed(ALICE), 10u128));

		assert_eq!(PerpetualAsset::total_collateral_balance(), 30u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 30u128);

		assert_noop!(
			PerpetualAsset::top_up_collateral(
				Origin::signed(ALICE),
				2_000_000_000_000_000_000u128,
			),
			orml_tokens::Error::<Runtime>::BalanceTooLow,
		);

		assert_eq!(PerpetualAsset::total_collateral_balance(), 30u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 30u128);
	});
}

#[test]
fn mint_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();

		assert_noop!(
			PerpetualAsset::mint(Origin::signed(ALICE), 10i128, 1u128),
			crate::Error::<Runtime>::PriceNotSet
		);

		PerpetualAsset::on_initialize(1);

		assert_noop!(
			PerpetualAsset::mint(
				Origin::signed(ALICE),
				2_000_000_000_000_000_000i128,
				2_000_000_000_000_000_000u128
			),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);
		assert_eq!(PerpetualAsset::margin(&ALICE), 0u128);

		assert_noop!(
			PerpetualAsset::mint(Origin::signed(ALICE), 10i128, 1u128),
			crate::Error::<Runtime>::NotEnoughIM
		);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 20u128));

		assert_eq!(
			last_event(),
			Event::perpetualasset(crate::Event::BalanceUpdated(ALICE, 100i128))
		);

		assert_eq!(PerpetualAsset::total_collateral_balance(), 20u128);
		assert_eq!(Tokens::total_balance(KUSD, &ALICE), 999_999_999_999_999_980u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 20u128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), -10i128, 0u128)); // Removes balance so no IM needed
		assert_eq!(PerpetualAsset::total_collateral_balance(), 20u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 20u128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 20i128, 2u128)); // Only 10 unit added, so 2 IM needed
		assert_eq!(PerpetualAsset::total_collateral_balance(), 22u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 22u128);

		// balance is now -200, so 40 IM needed, 22 already there, so need 18
		assert_noop!(
			PerpetualAsset::mint(Origin::signed(ALICE), -310i128, 17u128),
			crate::Error::<Runtime>::NotEnoughIM
		);
		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), -310i128, 18u128));
		assert_eq!(PerpetualAsset::total_collateral_balance(), 40u128);
		assert_eq!(PerpetualAsset::margin(&ALICE), 40u128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), -100i128, 20u128));
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

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), 100i128, 20u128));
		assert_ok!(PerpetualAsset::mint(Origin::signed(BOB), -100i128, 20u128));

		PerpetualAsset::on_initialize(2);

		assert_eq!(PerpetualAsset::inventory(&ALICE), 100i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(ALICE), -50i128, 0u128));
		PerpetualAsset::on_initialize(3);
		assert_eq!(PerpetualAsset::inventory(&ALICE), 50i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -50i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(CHARLIE), 100i128, 20u128));
		PerpetualAsset::on_initialize(4);
		assert_eq!(PerpetualAsset::inventory(&ALICE), 33i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 66i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -100i128);

		assert_ok!(PerpetualAsset::mint(Origin::signed(GEORGES), -100i128, 20u128));
		PerpetualAsset::on_initialize(4);
		assert_eq!(PerpetualAsset::inventory(&ALICE), 50i128);
		assert_eq!(PerpetualAsset::inventory(&CHARLIE), 100i128);
		assert_eq!(PerpetualAsset::inventory(&BOB), -75i128);
		assert_eq!(PerpetualAsset::inventory(&GEORGES), -75i128);
	});
}