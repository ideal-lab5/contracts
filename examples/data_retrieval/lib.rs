#![cfg_attr(not(feature = "std"), no_std)]
#![feature(trivial_bounds)]
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
//! ### unlock_data
//! 
//! Fetchs car addresses, executes each one, and submits a request to eject bytes from the network
//! 
//! 

use ink_env::Environment;
use ink_lang as ink;

/// Functions to interact with the Iris runtime as defined in runtime/src/lib.rs
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;

    #[ink(extension = 6, returns_result = false)]
    fn submit_results(caller: ink_env::AccountId, asset_id: u32, consumer: ink_env::AccountId, result: bool) -> [u8; 32];

    #[ink(extension = 7, returns_result = false)]
    fn request_bytes(asset_id: u32) -> [u8; 32];

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
            6 => Err(Self::FailSubmitResults),
            7 => Err(Self::FailRequestBytes),
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

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct RuleExecutor {

    }

    impl RuleExecutor {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self { }
            // INITIALIZE RULES
            // ink_lang::utils::initialize_contract(|contract: &mut Self| {
            //     contract.limit = limit;
            // })
        }

        #[ink(message, payable)]
        pub fn execute(&mut self, asset_id: u32) {      
            let contract_acct = self.env().account_id();
            let caller = self.env().caller();
            // Execute each rule
            
            // submit result
            self.env().extension().submit_results(contract_acct, asset_id.clone(), caller, true).map_err(|_| {}).ok();
            // call request bytes
            self.env().extension().request_bytes(asset_id.clone()).map_err(|_| {}).ok();
        }
    }
}
