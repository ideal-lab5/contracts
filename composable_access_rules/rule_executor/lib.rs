#![cfg_attr(not(feature = "std"), no_std)]
//! 
//! Data Retrieval Contract
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
//! Execute each composable access rule. In this case, we only execute the single use contract
//! 
//! 

use ink_env::Environment;
use ink_lang as ink;

/// Functions to interact with the Iris runtime as defined in runtime/src/lib.rs
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;

    #[ink(extension = 5, returns_result = false)]
    fn submit_results(caller: ink_env::AccountId, asset_id: u32, consumer: ink_env::AccountId, result: bool) -> [u8; 32];

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
            minimum_balance_rule_code_hash: Hash,
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
                minimum_balance_rule,

            }
        }

        #[ink(message)]
        pub fn execute(&mut self, asset_id: u32) {      
            let contract_acct = self.env().account_id();
            let caller = self.env().caller();
            single_use_result = self.single_use_rule.execute(asset_id, caller);
            self.env().emit_event(RuleExecuted{});
            let result = single_use_result;

            self.env()
                .extension()
                .submit_results(
                    contract_acct, 
                    asset_id.clone(), 
                    caller.clone(), 
                    result
                );
            self.env().emit_event(ResultsSubmitted{});
        }
    }
}
