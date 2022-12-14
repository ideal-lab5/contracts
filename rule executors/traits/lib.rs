// This file is part of Iris.
//
// Copyright (C) 2022 Ideal Labs.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

/// Trait definition for ComposableAccessRules
/// 
/// # Goal
/// 
/// Provide a trait that must be implemented by rules so they can be exeucted by a rule executor
/// 
#[ink::trait_definition]
pub trait ComposableAccessRule {
    /// Execute logic to determine if the caller is authorized to 
    /// fetch data associated with the asset id
    /// 
    /// * `asset_id`: The asset id to verify access to
    /// 
    #[ink(message)]
    fn execute(&mut self, asset_id: u32, consumer: ink_env::AccountId) -> bool;
}
