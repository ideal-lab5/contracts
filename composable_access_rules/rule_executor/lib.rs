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
use ink_prelude::string::String;

/// Functions to interact with the Iris runtime as defined in runtime/src/lib.rs
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;

    #[ink(extension = 5, returns_result = false)]
    fn submit_results(caller: ink_env::AccountId, consumer: ink_env::AccountId, asset_id: u32, public_key: String, result: bool) -> [u8; 32];

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
    use ink_storage::traits::SpreadAllocate;
    use ink_prelude::string::String;
    use limited_use_rule::LimitedUseRuleRef;
    use traits::ComposableAccessRule;

    #[ink(event)]
    pub struct ResultsSubmitted{}

    #[ink(event)]
    pub struct DataRequestSubmitted{}

    #[ink(event)]
    pub struct RuleExecuted{}

    #[ink(storage)]
    pub struct RuleExecutor {
        version: u32,
        single_use_rule: LimitedUseRuleRef,
    }

    impl RuleExecutor {
        #[ink(constructor)]
        pub fn new(
            version: u32,
            single_use_rule_code_hash: Hash,
        ) -> Self {
            // initialize rules
            let total_balance = Self::env().balance();
            let salt = version.to_le_bytes();
            // a token can be used only once
            let single_use_rule = LimitedUseRuleRef::new(1)
                .endowment(total_balance/4)
                .code_hash(single_use_rule_code_hash)
                .salt_bytes(salt)
                .instantiate()
                .unwrap_or_else(|error| {
                    panic!("failed at instantiating the Limited Use Rule contract: {:?}", error)
                });
            Self { 
                version,
                single_use_rule,

            }
        }

        /// Execute the rules specified in the executor
        /// 
        /// * `asset_id`: The asset id associated with the data to be accessed
        /// * `public_key`: An x25519 public key 
        /// 
        #[ink(message)]
        pub fn execute(&mut self, asset_id: u32, public_key: String) {      
            let contract_acct = self.env().account_id();
            let caller = self.env().caller();
            let single_use_result = self.single_use_rule.execute(asset_id, caller);
            self.env().emit_event(RuleExecuted{});
            let result = single_use_result;

            self.env()
                .extension()
                .submit_results(
                    contract_acct,
                    caller.clone(),
                    asset_id.clone(), 
                    public_key.clone(),
                    result
                );
            self.env().emit_event(ResultsSubmitted{});
        }
    }
}
