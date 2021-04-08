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

//! Unit tests for synthetics module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, ExtBuilder, Origin, Runtime, Synthetics, System, Tokens, ALICE, DOT, KUSD};

fn last_event() -> Event {
	System::events().last().unwrap().event.clone()
}

#[test]
fn create_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();

		assert_noop!(
			Synthetics::create(
				Origin::signed(ALICE),
				DOT,
				2_000_000_000_000_000_000u128,
				2_000_000_000_000_000_000u128
			),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);

		assert_noop!(
			Synthetics::create(Origin::signed(ALICE), DOT, 10u128, 1u128),
			crate::Error::<Runtime>::NotEnoughIM
		);

		assert_ok!(Synthetics::create(Origin::signed(ALICE), DOT, 10u128, 2u128));

		assert_eq!(Synthetics::total_collateral_balance(), 2u128);
		assert_eq!(Tokens::total_balance(KUSD, &ALICE), 999_999_999_999_999_998u128);
		assert_eq!(Synthetics::collateral_balance_of(&DOT, &ALICE), 2u128);

		assert_eq!(
			last_event(),
			Event::synthetics(crate::Event::ShortBalanceUpdated(ALICE, 10i128))
		);
	});
}

#[test]
fn buy_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();

		assert_noop!(
			Synthetics::buy(
				Origin::signed(ALICE),
				DOT,
				2_000_000_000_000_000_000u128,
				2_000_000_000_000_000_000u128
			),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);

		assert_noop!(
			Synthetics::buy(Origin::signed(ALICE), DOT, 10u128, 1u128),
			crate::Error::<Runtime>::NotEnoughIM
		);

		assert_ok!(Synthetics::buy(Origin::signed(ALICE), DOT, 10u128, 2u128));

		assert_eq!(Synthetics::total_collateral_balance(), 2u128);
		assert_eq!(Tokens::total_balance(KUSD, &ALICE), 999_999_999_999_999_998u128);
		assert_eq!(Synthetics::collateral_balance_of(&DOT, &ALICE), 2u128);

		assert_eq!(
			last_event(),
			Event::synthetics(crate::Event::LongBalanceUpdated(ALICE, 10i128))
		);
	});
}