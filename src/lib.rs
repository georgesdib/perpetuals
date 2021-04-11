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
// TODO: make documentation better
// TODO: clean up code
// TODO: check redeeming and the updating of inventory

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use primitives::{Amount, Balance, CurrencyId};
use sp_runtime::{traits::AccountIdConversion, FixedPointNumber, ModuleId};
use sp_arithmetic::Perquintill;
use sp_std::{convert::TryInto, result};
use support::Price;

mod mock;
mod tests;

pub use module::*;

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
		/// Emitted when claiming the wrong sign, ie buy vs sell
		WrongSign,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Emitted when collateral is updated by \[Amount\]
		CollateralUpdated(Amount),
		/// Emitted when the balance of \[T::AccountId\] is updated to \[Balance\]
		BalanceUpdated(T::AccountId, Amount),
	}

	#[pallet::storage]
	#[pallet::getter(fn balances)]
	pub(crate) type Balances<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Amount, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn inventory)]
	pub(crate) type Inventory<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Amount, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn margin)]
	pub(crate) type Margin<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Amount, ValueQuery>;

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
			// TODO: Get price from Oracle
			Self::update_margin(&1.into());
			Self::liquidate();
			Self::match_interest();
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
			collateral: Amount,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let current_balance = Balances::<T>::try_get(who.clone()).unwrap_or(0.into());
			let balance = current_balance.checked_add(amount).ok_or(Error::<T>::Overflow)?;
			let positive_collateral = Self::balance_try_from_amount_abs(collateral)?;

			// Check if enough collateral
			let current_margin = Margin::<T>::try_get(who.clone()).unwrap_or(0i128.into());
			let price = Price0::<T>::get().ok_or(Error::<T>::PriceNotSet)?;
			let positive_balance = Self::balance_try_from_amount_abs(balance)?;
			let total_price = price.checked_mul_int(positive_balance).ok_or(Error::<T>::Overflow)?;
			let needed_im = Self::amount_try_from_balance(total_price / T::InitialIMDivider::get())?;
			let new_margin = current_margin.checked_add(collateral).ok_or(Error::<T>::Overflow)?;
			if new_margin < needed_im {
				return Err(Error::<T>::NotEnoughIM.into());
			}

			let module_account = Self::account_id();

			if collateral > 0 {
				// Transfer the collateral to the module's account
				T::Currency::transfer(T::NativeCurrencyId::get(), &who, &module_account, positive_collateral)?;
			}

			if collateral < 0 {
				// Transfer the collateral from the module's account
				T::Currency::transfer(T::NativeCurrencyId::get(), &module_account, &who, positive_collateral)?;
			}

			if collateral != 0 {
				Margin::<T>::insert(who.clone(), new_margin);
				Self::deposit_event(Event::CollateralUpdated(collateral));
			}

			// Update the balances
			Balances::<T>::insert(who.clone(), balance);
			Self::deposit_event(Event::BalanceUpdated(who, balance));

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Call *M* the total margin for a participant *A*,
	/// Call *T* the total interest, and *B* the inventory (open interest is $T - B$)
	/// The needed collateral for maintaining the inventory is $B * P_0 * L$
	/// If $B * P_0 * L > M$, then liquididate the inventory as per below.
	/// If $B * P_0 * L < M$, but $T * P_0 * L > M$ then close out part of the total interest such that:
	/// $$
	/// I * P_0 * T' = M \\
	/// T' >= B
	/// $$
	/// If such $T'$ is possible, total interest becomes $T' = M / (I * P_0)$
	/// and inventory remains at *B*. If no such $T'$ is possible
	/// which would be the case if $M / (I * P_0) < B$ or $M < B * I * P_0$
	/// then liquidate all the open interest, so total interest becomes $T' = B$
	/// and inventory remains at *B*
	/// This is done to make sure that if an opposing open interest comes during that block
	/// it does not suffer from immediate liquidation.
	/// 
	/// ### Liquidation of inventory
	/// If $B * P_0 * L > M$, liquidate the full position
	/// so total position and inventory goes to $0$
	fn liquidate() {
		// TODO: add unittests
		let price = Price0::<T>::get().unwrap(); // Price was already set so never panics
		let liq_div = T::LiquidationDivider::get();
		let im_div = T::InitialIMDivider::get();

		for (account, margin) in Margin::<T>::iter() {
			let inventory_signed = Self::inventory(account.clone());
			let inventory = Self::balance_try_from_amount_abs(inventory_signed).unwrap(); // TODO handle overflow better
			let balance = Self::balance_try_from_amount_abs(
				Balances::<T>::get(account.clone())).unwrap(); // TODO handle overflow better

			if margin <= 0 { // No Collateral left, liquidate
				Balances::<T>::insert(account.clone(), 0);
				Inventory::<T>::insert(account, 0);
			} else {

				let positive_margin = Self::balance_try_from_amount_abs(margin).unwrap(); // TODO panics if error

				if (price.saturating_mul_int(balance) / liq_div) > positive_margin {
					// TODO: make sure no division by 0, but that should be obvious, maybe check at genesis
					let a = price.saturating_mul_int(inventory);
					let threshold = a / liq_div;
					if threshold < positive_margin {
						let b = a / im_div;
						if b > positive_margin {
							Balances::<T>::insert(account, inventory_signed);
						} else {
							let mut new_balance = price.saturating_div_int(im_div);
							new_balance = positive_margin / new_balance;
							// TODO: handle overflow better
							let mut n: Amount = Self::amount_try_from_balance(new_balance).unwrap();
							if inventory_signed < 0 {
								n *= -1;
							}
							Balances::<T>::insert(account, n);
						}
					} else {
						Balances::<T>::insert(account.clone(), 0);
						Inventory::<T>::insert(account, 0);
					}
				}
			}
		}
	}

	/// If $\forall i, X_i = 0$ then no interest to match. Otherwise, call $R = \frac{\sum_i Y_i}{\sum_i X_i}$
	/// $B_i$ has bought $min(X_i, X_i * R)$
	/// $S_i$ has sold $min(Y_i, Y_i / R)$
	fn match_interest() {
		// Reset inventory
		Inventory::<T>::remove_all();
		let mut shorts: Balance = 0u128;
		let mut longs: Balance = 0u128;
		for balance in Balances::<T>::iter_values() {
			let b = Self::balance_try_from_amount_abs(balance).unwrap(); // Panics if error
			if balance < 0 {
				shorts += b;
			} else {
				longs += b;
			}
		}

		// If one of them is 0, nothing to match
		if shorts != 0 && longs != 0 {
			let ratio;
			let shorts_filled;
			if shorts < longs {
				ratio = Perquintill::from_rational(shorts, longs);
				shorts_filled = true;
			} else {
				ratio = Perquintill::from_rational(longs, shorts);
				shorts_filled = false;
			}
			for (account, balance) in Balances::<T>::iter() {
				let mut amount: Amount;
				if (balance < 0 && shorts_filled) || (balance >= 0 && !shorts_filled) {
					amount = balance;
				} else {
					let b = Self::balance_try_from_amount_abs(balance).unwrap(); // Panics if error
					amount = Self::amount_try_from_balance(ratio.mul_floor(b)).unwrap(); // Should never fail given we know no overflow
					if balance < 0 {
						amount *= -1;
					}
				}
				Inventory::<T>::insert(account, amount);
			}
		}
	}

	fn update_margin(new_price: &Price) {
		let p0 = Price0::<T>::get().unwrap_or(*new_price);
		let delta = *new_price - p0;
		Price0::<T>::set(Some(*new_price));
		if !delta.is_zero() {
			Margin::<T>::translate(|account, margin: Amount| -> Option<Amount> {
				let bal = Balances::<T>::get(account);
				let update_balance = delta.checked_mul_int(bal)?;
				Some(margin + update_balance)
			});
		}
	}

	fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	/// Gets the total balance of collateral in NativeCurrency
	pub fn total_collateral_balance() -> Balance {
		T::Currency::total_balance(T::NativeCurrencyId::get(), &Self::account_id())
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
