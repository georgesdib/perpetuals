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

//! # Sythetics Module
//!
//! ## Overview
//!
//! Price any synthetic payoff as long as oracle can provide a price

// TODO: add weight stuff, and benchmark it

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
fn get_price(_curreny: &CurrencyId) -> Price {
	1.into()
}

// TODO: take that from somewhere else
fn get_collateral_divider(_currency: &CurrencyId) -> u128 {
	5
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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Emitted when collateral is updated by \[amount\]
		CollateralUpdated(Amount),
		/// Emitted when the short balance of \[T::AccountId\] is updated by
		/// \[Amount\]
		ShortBalanceUpdated(T::AccountId, Amount),
		/// Emitted when the long balance of \[T::AccountId\] is updated by
		/// \[Amount\]
		LongBalanceUpdated(T::AccountId, Amount),
	}

	#[pallet::storage]
	type Shorts<T: Config> = StorageMap<_, Twox128, (CurrencyId, T::AccountId), Balance, ValueQuery>;

	#[pallet::storage]
	type Longs<T: Config> = StorageMap<_, Twox128, (CurrencyId, T::AccountId), Balance, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn margin)]
	pub(crate) type Margin<T: Config> = StorageMap<_, Twox128, (CurrencyId, T::AccountId), Balance, ValueQuery>;

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
			10
		}

		fn on_finalize(_n: T::BlockNumber) {}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1000)]
		/// Create a payoff (or append to existing)
		/// - `origin`: the calling account
		/// - `currency`: the asset to be priced
		/// - `supply`: the amount of asset to be minted
		/// - `collateral`: the amount of collateral in native currency
		pub(super) fn create(
			origin: OriginFor<T>,
			currency: CurrencyId,
			supply: Balance,
			collateral: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let s = Self::amount_try_from_balance(supply)?;
			let who_clone = who.clone();

			Self::transact(&who, &currency, &supply, &collateral)?;

			// Update the shorts balances
			Shorts::<T>::insert((currency, who), supply);
			Self::deposit_event(Event::ShortBalanceUpdated(who_clone, s));

			Ok(().into())
		}

		#[pallet::weight(1000)]
		/// Buys a payoff
		/// - `origin`: the calling account
		/// - `currency`: the asset to be priced
		/// - `amount`: the amount of asset to be bought
		/// - `collateral`: the amount of collateral in native currency
		pub(super) fn buy(
			origin: OriginFor<T>,
			currency: CurrencyId,
			amount: Balance,
			collateral: Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let s = Self::amount_try_from_balance(amount)?;
			let who_clone = who.clone();

			Self::transact(&who, &currency, &amount, &collateral)?;

			// Update the longs balances
			Longs::<T>::insert((currency, who), amount);
			Self::deposit_event(Event::LongBalanceUpdated(who_clone, s));

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn check_collateral(
		currency: &CurrencyId,
		quantity: &Balance,
		collateral: &Balance,
	) -> result::Result<(), Error<T>> {
		let total_price = get_price(currency)
			.checked_mul_int(*quantity)
			.ok_or(Error::<T>::Overflow)?;
		let needed_im = total_price / get_collateral_divider(currency);
		if *collateral < needed_im {
			return Err(Error::<T>::NotEnoughIM.into());
		}
		Ok(())
	}

	fn transact(
		who: &T::AccountId,
		currency: &CurrencyId,
		amount: &Balance,
		collateral: &Balance,
	) -> DispatchResultWithPostInfo {
		// Ensure enough collateral
		Self::check_collateral(currency, amount, collateral)?;

		let module_account = Self::account_id();
		let col = Self::amount_try_from_balance(*collateral)?;
		let who_clone = (*who).clone();

		// Transfer the collateral to the module's account
		T::Currency::transfer(T::NativeCurrencyId::get(), who, &module_account, *collateral)?;
		Margin::<T>::insert((*currency, who_clone), *collateral);
		Self::deposit_event(Event::CollateralUpdated(col));

		Ok(().into())
	}

	fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	/// Gets the total balance of collateral in NativeCurrency
	pub fn total_collateral_balance() -> Balance {
		T::Currency::total_balance(T::NativeCurrencyId::get(), &Self::account_id())
	}

	/// Gets the collateral balance of collateral of \[AccountId\] in
	/// \[CurrencyId\]
	pub fn collateral_balance_of(currency: &CurrencyId, who: &T::AccountId) -> Balance {
		Self::margin((currency, who))
	}

	/// Convert `Balance` to `Amount`.
	fn amount_try_from_balance(b: Balance) -> result::Result<Amount, Error<T>> {
		TryInto::<Amount>::try_into(b).map_err(|_| Error::<T>::AmountConvertFailed)
	}

	/// Convert the absolute value of `Amount` to `Balance`.
	fn _balance_try_from_amount_abs(a: Amount) -> result::Result<Balance, Error<T>> {
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
