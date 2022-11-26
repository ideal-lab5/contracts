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
//! 
//! Rule Executor Contract
//! 
//! # Goal
//! This contract allows data consumers to unlock data for which
//! composable access rules have been specified. It accomplishes this by retrieving any composable access rules 
//! associated with a given data asset class and executing each one. Post execution, the contract submits a call
//! to request bytes from the network (which is then processed by a proxy node)
//! 
//! ## Functions
//! 
//! ### execute
//! 
//! Execute each composable access rule. In this case, we only execute the single use rule.
//! After execution, report results on chain
//! 
//! 

use ink_env::Environment;
use ink_lang as ink;

/// Functions to interact with the Iris runtime as defined in runtime/src/lib.rs
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;

    #[ink(extension = 5, returns_result = false)]
    fn submit_results(caller: ink_env::AccountId, consumer: ink_env::AccountId, asset_id: u32, result: bool, public_key: [u8;32]) -> [u8; 32];

}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IrisErr {
    FailSubmitResults,
    FailRequestBytes,
}

impl ink_env::chain_extension::FromStatusCode for IrisErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            5 => Err(Self::FailSubmitResults),
            _ => panic!("encountered unknown status code {:?}", status_code),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum CustomEnvironment {}

impl Environment for CustomEnvironment {
    const MAX_EVENT_TOPICS: usize =
        <ink_env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;
    type Balance = <ink_env::DefaultEnvironment as Environment>::Balance;
    type Hash = <ink_env::DefaultEnvironment as Environment>::Hash;
    type BlockNumber = <ink_env::DefaultEnvironment as Environment>::BlockNumber;
    type Timestamp = <ink_env::DefaultEnvironment as Environment>::Timestamp;
    type ChainExtension = Iris;
}

#[ink::contract(env = crate::CustomEnvironment)]
mod rule_executor {

    #[ink(event)]
    pub struct ResultsSubmitted{
        pub public_key: [u8;32],
    }

    #[ink(storage)]
    pub struct RuleExecutor {
        version: u32,
    }

    impl RuleExecutor {
        #[ink(constructor)]
        pub fn new(
            version: u32,
        ) -> Self {
            Self { version }
        }

        /// Execute the rules specified in the executor
        /// 
        /// * `asset_id`: The asset id associated with the data to be accessed
        /// * `public_key`: An x25519 public key 
        /// 
        #[ink(message, payable)]
        pub fn execute(&mut self, asset_id: u32, public_key: [u8;32]) {
            let contract_acct = self.env().account_id();
            let caller = self.env().caller();
            self.env()
                .extension()
                .submit_results(
                    contract_acct,
                    caller,
                    asset_id,
                    true,
                    public_key,
                );
            self.env().emit_event(ResultsSubmitted{ public_key });
        }
    }
}
