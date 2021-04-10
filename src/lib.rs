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

//! # PerpetualAsset Module
//!
//! ## Overview
//!
//! Given an asset for which an Oracle can provide a price, give a way
//! for longs and shorts to express their view

// TODO: add weight stuff, and benchmark it
// TODO: allow any sort of payoff

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use primitives::{Amount, Balance, CurrencyId};
use sp_runtime::{traits::AccountIdConversion, FixedPointNumber, ModuleId};
use sp_std::{convert::TryInto, result};
use support::Price;

mod mock;
mod tests;

pub use module::*;

// TODO: take that from oracle
fn get_price() -> Price {
	1.into()
}

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The synthetic's module id, keep all collaterals.
		#[pallet::constant]
		type ModuleId: Get<ModuleId>;

		/// The asset to be priced
		#[pallet::constant]
		type CurrencyId: Get<CurrencyId>;

		/// Initial IM Divider
		#[pallet::constant]
		type InitialIMDivider: Get<Balance>;

		/// Liquidation Divider
		#[pallet::constant]
		type LiquidationDivider: Get<Balance>;

		/// Currency for transfer currencies
		type Currency: MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId, Balance = Balance>;

		/// The native currency to pay in.
		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not enough IM is sent
		NotEnoughIM,
		/// Fail to convert from Amount to Balance and vice versa
		AmountConvertFailed,
		/// Overflow
		Overflow,
		/// Emitted when trying to redeem without enough balance
		NotEnoughBalance,
		/// Emitted when P0 not set
		PriceNotSet,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Emitted when collateral is updated by \[Balance\]
		CollateralUpdated(Balance),
		/// Emitted when the balance of \[T::AccountId\] is updated to \[Balance\]
		BalanceUpdated(T::AccountId, Amount),
	}

	#[pallet::storage]
	pub(crate) type Balances<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Amount, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn margin)]
	pub(crate) type Margin<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Balance, ValueQuery>;

	#[pallet::storage]
	pub(crate) type Price0<T: Config> = StorageValue<_, Price>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			GenesisConfig {}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			Self::update_margin();
			// TODO what the hell is this??
			10
		}

		fn on_finalize(_n: T::BlockNumber) {}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1000)]
		/// Mints the payoff
		/// - `origin`: the calling account
		/// - `amount`: the amount of asset to be minted(can be positive or negative)
		/// - `collateral`: the amount of collateral in native currency
		pub(super) fn mint(
			origin: OriginFor<T>,
			amount: Amount,
			collateral: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let current_balance = Balances::<T>::try_get(who.clone()).unwrap_or(0.into());
			let balance = current_balance.checked_add(amount).ok_or(Error::<T>::Overflow)?;

			// Check if enough collateral
			let current_margin = Margin::<T>::try_get(who.clone()).unwrap_or(0u128.into());
			let price = Price0::<T>::get().ok_or(Error::<T>::PriceNotSet)?;
			let positive_balance = Self::balance_try_from_amount_abs(balance)?;
			let total_price = price.checked_mul_int(positive_balance).ok_or(Error::<T>::Overflow)?;
			let needed_im = total_price / Self::get_collateral_divider();
			if current_margin + collateral < needed_im {
				return Err(Error::<T>::NotEnoughIM.into());
			}

			let module_account = Self::account_id();

			if collateral > 0 {
				// Transfer the collateral to the module's account
				T::Currency::transfer(T::NativeCurrencyId::get(), &who, &module_account, collateral)?;
				Margin::<T>::insert(who.clone(), current_margin + collateral);
			}

			Self::deposit_event(Event::CollateralUpdated(collateral));

			// Update the balances
			Balances::<T>::insert(who.clone(), balance);
			Self::deposit_event(Event::BalanceUpdated(who, balance));

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	// TODO: add unittests
	fn update_margin() {
		let p1 = get_price();
		let p0 = Price0::<T>::get().unwrap_or(p1);
		let delta = p1 - p0;
		Price0::<T>::set(Some(p1));
		if !delta.is_zero() {
			Margin::<T>::translate(|account, margin: Balance| -> Option<Balance> {
				let bal = Balances::<T>::get(account); // This should never fail, TODO handle that
				// TODO handle overflow better
				let update_balance = delta.saturating_mul_int(bal);
				// TODO handle better the failure here
				let amount = Self::amount_try_from_balance(margin).unwrap(); // panic if this fails
				let mut res = amount + update_balance;
				if res < 0 {
					res = 0; // No more margin left, account will be liquidated
				}
				Some(Self::balance_try_from_amount_abs(res).unwrap())
			});
		}
	}

	fn get_collateral_divider() -> Balance {
		T::InitialIMDivider::get()
	}

	fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	/// Gets the total balance of collateral in NativeCurrency
	pub fn total_collateral_balance() -> Balance {
		T::Currency::total_balance(T::NativeCurrencyId::get(), &Self::account_id())
	}

	/// Gets the collateral balance of collateral of \[AccountId\]
	pub fn collateral_balance_of(who: &T::AccountId) -> Balance {
		Self::margin(who)
	}

	/// Convert `Balance` to `Amount`.
	fn amount_try_from_balance(b: Balance) -> result::Result<Amount, Error<T>> {
		TryInto::<Amount>::try_into(b).map_err(|_| Error::<T>::AmountConvertFailed)
	}

	/// Convert the absolute value of `Amount` to `Balance`.
	fn balance_try_from_amount_abs(a: Amount) -> result::Result<Balance, Error<T>> {
		TryInto::<Balance>::try_into(a.saturating_abs()).map_err(|_| Error::<T>::AmountConvertFailed)
	}
}

#[cfg(feature = "std")]
impl GenesisConfig {
	/// Direct implementation of `GenesisBuild::build_storage`.
	///
	/// Kept in order not to break dependency.
	pub fn build_storage<T: Config>(&self) -> Result<sp_runtime::Storage, String> {
		<Self as GenesisBuild<T>>::build_storage(self)
	}

	/// Direct implementation of `GenesisBuild::assimilate_storage`.
	///
	/// Kept in order not to break dependency.
	pub fn assimilate_storage<T: Config>(&self, storage: &mut sp_runtime::Storage) -> Result<(), String> {
		<Self as GenesisBuild<T>>::assimilate_storage(self, storage)
	}
}
