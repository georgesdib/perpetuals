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

//! Mocks for perpetualasset module.

#![cfg(test)]

use super::*;
use frame_support::{construct_runtime, pallet_prelude::GenesisBuild, parameter_types};
use orml_traits::parameter_type_with_key;
use primitives::TokenSymbol;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup};
use sp_std::cell::RefCell;

pub type BlockNumber = u64;
pub type AccountId = u128;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const GEORGES: AccountId = 4;
pub const KUSD: CurrencyId = CurrencyId::Token(TokenSymbol::KUSD);
pub const DOT: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);

mod perpetualasset {
	pub use super::super::*;
}

parameter_types!(
	pub const BlockHashCount: BlockNumber = 250;
	pub const PerpetualAssetModuleId: PalletId = PalletId(*b"aca/pasm");
	pub const NativeCurrencyId: CurrencyId = KUSD;
	pub const UsedCurrencyId: CurrencyId = DOT;
	pub const InitialIMDivider: Balance = 5u128;
	pub const LiquidationDivider: Balance = 10u128;
);

impl frame_system::Config for Runtime {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
}

thread_local! {
	static PRICE: RefCell<Option<Price>> = RefCell::new(Some(Price::one()));
}

pub struct MockPriceSource;

impl MockPriceSource {
	pub fn set_price(price: Option<Price>) {
		PRICE.with(|v| *v.borrow_mut() = price);
	}
}

impl PriceProvider<CurrencyId> for MockPriceSource {
	fn get_relative_price(_base: CurrencyId, _quote: CurrencyId) -> Option<Price> {
		None
	}

	fn get_price(_currency_id: CurrencyId) -> Option<Price> {
		PRICE.with(|v| *v.borrow_mut())
	}

	fn lock_price(_currency_id: CurrencyId) {}

	fn unlock_price(_currency_id: CurrencyId) {}
}

impl perpetualasset::Config for Runtime {
	type Event = Event;
	type PalletId = PerpetualAssetModuleId;
	type Currency = Tokens;
	type NativeCurrencyId = NativeCurrencyId;
	type CurrencyId = UsedCurrencyId;
	type InitialIMDivider = InitialIMDivider;
	type LiquidationDivider = LiquidationDivider;
	type PriceSource = MockPriceSource;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Event<T>},
		PerpetualAsset: perpetualasset::{Pallet, Call, Event<T>, Config, Storage},
		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
	}
);

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![
				(ALICE, KUSD, 1_000_000_000_000_000_000u128),
				(BOB, KUSD, 1_000_000_000_000_000_000u128),
				(CHARLIE, KUSD, 1_000_000_000_000_000_000u128),
				(GEORGES, KUSD, 1_000_000_000_000_000_000u128),
			],
		}
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		perpetualasset::GenesisConfig::default()
			.assimilate_storage::<Runtime>(&mut t)
			.unwrap();

		t.into()
	}
}
