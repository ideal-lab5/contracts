#![cfg_attr(not(feature = "std"), no_std)]
#![feature(trivial_bounds)]
//!
//! Limited Use Asset Composable Access Rule
//! 
//! This contract allows data owners to impose limitations on the number
//! of times an address may use a token to access data associated with the 
//! asset class
//! 
//! 

use ink_env::Environment;
use ink_lang as ink;

// #[ink::chain_extension]
// pub trait Iris {
//     type ErrorCode = IrisErr;

//     #[ink(extension = 6, returns_result = true)]
//     fn check_owner(query_address: ink_env::AccountId, asset_id: u32) -> bool;
// }


/// Functions to interact with the Iris runtime as defined in runtime/src/lib.rs
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;
    
    #[ink(extension = 6, returns_result = false)]
    fn query_owner(query: ink_env::AccountId, asset_id: u32) -> bool;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IrisErr {
    FailQueryOwner,
}

impl ink_env::chain_extension::FromStatusCode for IrisErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            6 => Err(Self::FailQueryOwner),
            _ => panic!("encountered unknown status code"),
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
mod limited_use_rule {
    use ink_storage::traits::SpreadAllocate;
    use traits::ComposableAccessRule;

    #[ink(event)]
    pub struct RegistrationSuccessful{}

    #[ink(event)]
    pub struct AlreadyRegistered{}

    #[ink(event)]
    pub struct CallerIsNotOwner{}

    #[ink(event)]
    pub struct ExecutionSuccessful{}

    #[ink(event)]
    pub struct ExecutionFailed{}

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct LimitedUseRuleContract {
        asset_registry: ink_storage::Mapping<u32, AccountId>,
        usage_counter: ink_storage::Mapping<AccountId, u32>,
    }

    impl LimitedUseRuleContract {
        #[ink(constructor)]
        pub fn new(initial_supply: Balance) -> Self {
            ink_lang::utils::initialize_contract(|_| {})
        }
    }

    impl ComposableAccessRule for LimitedUseRuleContract {

        #[ink(message, payable)]
        fn register(&mut self, asset_id: u32) {
            let caller = self.env().caller();
            if let Some(limit) = self.asset_registry.get(&asset_id) {
                self.env().emit_event(AlreadyRegistered);
            } else {
                // check that caller is asset owner
                let asset_owner = self.env()
                    .extension()
                    .check_owner(asset_id)?;
                if (asset_owner == caller) {
                    self.asset_registry.insert(&asset_id, &caller);
                    self.env().emit_event(RegistrationSuccessful);
                } else {
                    self.env().emit_event(CallerIsNotOwner);
                }
            }
        }

        #[ink(message, payable)]
        fn execute(&mut self, asset_id: u32) {
            // let caller = self.env().caller();
            // // get count for the asset id
            // let access_limit = self.asset_registry.get(&asset_id);
            // // if let Some(self.usage_counter)
        }
    }

    // #[cfg(test)]
    // mod tests {
    //     use super::*;
    //     use ink_lang as ink;

    //     #[ink::test]
    //     fn can_register_new_asset_positive_limit() {
    //         let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
    //         struct MockExtension;
    //         impl ink_env::test::ChainExtension for MockExtension {
    //             fn func_id(&self) -> u32 {
    //                 6
    //             }
    //             fn call(&mut self, _input: &[u8], output: &mut AccountId) -> u32 {
    //                 // let ret: AccountId = AccountId::from([0x01; 32]);
    //                 let ret: AccountId = accounts.alice;
    //                 scale::Encode::encode_to(&ret, output);
    //                 0
    //             }
    //             ink_env::test::register_chain_extension(MockExtension);
    //         }

    //         let limited_use_contract = LimitedUseRuleContract::new();
    //         ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.alice);
    //         limited_use_contract.register(1);
    //     }

    //     // #[ink::test]
    //     // fn cant_register_new_asset_with_negative_limit() {

    //     // }
        
    //     // #[ink::test]
    //     // fn cant_register_new_asset_with_zero_limit() {

    //     // }

    //     // #[ink::test]
    //     // fn cant_register_new_asset_when_not_owner() {

    //     // }
    // }
}