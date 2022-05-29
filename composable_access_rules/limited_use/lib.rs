#![cfg_attr(not(feature = "std"), no_std)]
#![feature(trivial_bounds)]
//!
//! Limited Use Rule
//! 
//! # Goal
//! This contract allows data owners to impose limitations on the number
//! of times an address may use a token to access data associated with the 
//! asset class
//! 
//! # Register
//! The asset registry maps the asset id to the owner (can probably remove owner?)
//! 
//! # Execute
//! 
//! 

use ink_env::Environment;
use ink_lang as ink;

/// Functions to interact with the Iris runtime as defined in runtime/src/lib.rs
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;

    #[ink(extension = 2, returns_result = false)]
    fn burn(caller: ink_env::AccountId, target: ink_env::AccountId, asset_id: u32, amount: u64) -> [u8; 32];
    
    #[ink(extension = 5, returns_result = false, handle_status = false)]
    fn query_owner(query: ink_env::AccountId, asset_id: u32) -> bool;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IrisErr {
    FailBurnAsset,
    FailQueryOwner,
}

impl ink_env::chain_extension::FromStatusCode for IrisErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            2 => Err(Self::FailBurnAsset),
            5 => Err(Self::FailQueryOwner),
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

    #[ink(event)]
    pub struct AssetNotRegistered{}

    #[ink(event)]
    pub struct LimitExceeded{}

    #[ink(event)]
    pub struct AccessAllowed{}

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct LimitedUseRuleContract {
        limit: u32,
        asset_registry: ink_storage::Mapping<u32, AccountId>,
        usage_counter: ink_storage::Mapping<AccountId, u32>,
    }

    impl LimitedUseRuleContract {
        #[ink(constructor)]
        pub fn new(limit: u32) -> Self {
            if limit <= 0 {
                panic!("limit must be positive");
            }
            ink_lang::utils::initialize_contract(|contract: &mut Self| {
                contract.limit = limit;
            })
        }

        fn get_limit(&self) -> u32 {
            self.limit
        }
    }

    impl ComposableAccessRule for LimitedUseRuleContract {

        /// register the asset id in the limited use rule instance
        #[ink(message, payable)]
        fn register(&mut self, asset_id: u32) {
            let caller = self.env().caller();
            if let Some(admin) = self.asset_registry.get(&asset_id) {
                self.env().emit_event(AlreadyRegistered{});
            } else {
                // check that caller is asset owner
                let is_owner = self.env()
                    .extension()
                    .query_owner(caller, asset_id);
                
                if is_owner {
                    self.asset_registry.insert(&asset_id, &caller);
                    self.env().emit_event(RegistrationSuccessful{});
                } else {
                    self.env().emit_event(CallerIsNotOwner{});
                }
            }
        }

        /// check if the number of times a caller has attempted access to the asset 
        /// exceeds the pre-defined limit amoutn
        /// 
        /// * `asset_id`: The asset to which access is attempted
        /// 
        #[ink(message, payable)]
        fn execute(&mut self, asset_id: u32) {
            let caller = self.env().caller();
            // if the asset has been registered
            if let Some(owner) = self.asset_registry.get(&asset_id) {
                // check number of times the caller has attempted access
                if let Some(access_attempts) = self.usage_counter.get(&caller) {
                    if access_attempts < self.limit {
                        let next_attempt_count = access_attempts + 1;
                        self.usage_counter.insert(&caller, &next_attempt_count);
                        self.env().emit_event(AccessAllowed{});
                    } else {
                        self.env()
                            .extension()
                            .burn(
                                caller, caller, asset_id, 1,
                            )
                            .map_err(|_| {});
                        self.env().emit_event(LimitExceeded{});
                    }
                } else {
                    // first access
                    self.usage_counter.insert(&caller, &1);
                }
            } else {
                self.env().emit_event(AssetNotRegistered{});
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;

        #[ink::test]
        fn can_create_new_contract_with_positive_limit() {
            let limit = 10;
            let limited_use_contract = LimitedUseRuleContract::new(limit);
            assert_eq!(limit, limited_use_contract.get_limit());
        }

        /**
         * # Tests for the `register` function
         */

        fn setup_test(limit: u32, default_account: ink_env::AccountId) -> LimitedUseRuleContract {
            let limited_use_contract = LimitedUseRuleContract::new(limit);
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(default_account);
            limited_use_contract
        }

        fn mock_query_owner_extension_true() {
            struct MockExtension;
            impl ink_env::test::ChainExtension for MockExtension {
                fn func_id(&self) -> u32 {
                    5
                }
                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    // let ret: AccountId = AccountId::from([0x01; 32]);
                    scale::Encode::encode_to(&true, output);
                    5
                }
            }

            ink_env::test::register_chain_extension(MockExtension);
        }

        fn mock_query_owner_extension_false() {
            struct MockExtension;
            impl ink_env::test::ChainExtension for MockExtension {
                fn func_id(&self) -> u32 {
                    5
                }
                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    // let ret: AccountId = AccountId::from([0x01; 32]);
                    scale::Encode::encode_to(&false, output);
                    5
                }
            }

            ink_env::test::register_chain_extension(MockExtension);
        }

        fn mock_burn_extension() {
            struct MockBurnExtension;
            impl ink_env::test::ChainExtension for MockBurnExtension {
                fn func_id(&self) -> u32 {
                    2
                }
                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    let ret: [u8; 32] = [1; 32];
                    scale::Encode::encode_to(&ret, output);
                    2
                }
            }

            ink_env::test::register_chain_extension(MockBurnExtension);
        }

        #[ink::test]
        fn can_register_new_asset_when_owner() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut limited_use_contract = setup_test(1, accounts.alice);
            mock_query_owner_extension_true();

            limited_use_contract.register(1);
            assert_eq!(Some(accounts.alice), limited_use_contract.asset_registry.get(1));
        }

        #[ink::test]
        fn cant_register_new_asset_when_not_owner() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut limited_use_contract = setup_test(1, accounts.alice);
            mock_query_owner_extension_false();

            limited_use_contract.register(1);
            assert_eq!(None, limited_use_contract.asset_registry.get(1));
        }
        
        #[ink::test]
        fn can_execute_and_increment_on_first_access() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut limited_use_contract = setup_test(2, accounts.alice);
            mock_query_owner_extension_true();
            mock_burn_extension();

            // GIVEN: An asset class is registered
            limited_use_contract.register(1);
            assert_eq!(Some(accounts.alice), limited_use_contract.asset_registry.get(1));

            // WHEN: I attempt to invoke the execute function
            limited_use_contract.execute(1);
            // THEN: The access attempt value is incremented by one
            assert_eq!(Some(1), limited_use_contract.usage_counter.get(accounts.alice));
        }

        #[ink::test]
        fn can_execute_and_increment_on_second_access() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut limited_use_contract = setup_test(2, accounts.alice);
            mock_query_owner_extension_true();

            // GIVEN: An asset class is registered
            limited_use_contract.register(1);
            assert_eq!(Some(accounts.alice), limited_use_contract.asset_registry.get(1));

            // WHEN: I attempt to invoke the execute function
            limited_use_contract.execute(1);
            // THEN: The access attempt value is incremented by one
            assert_eq!(Some(1), limited_use_contract.usage_counter.get(accounts.alice));

            // WHEN: I attempt to invoke the execute function AGAIN
            limited_use_contract.execute(1);
            // THEN: The access attempt value is incremented by one
            assert_eq!(Some(2), limited_use_contract.usage_counter.get(accounts.alice));
        }

        #[ink::test]
        fn can_execute_and_not_increment_and_burn_when_limit_exceeded() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut limited_use_contract = setup_test(2, accounts.alice);
            mock_query_owner_extension_true();
            mock_burn_extension();
            // GIVEN: An asset class is registered
            limited_use_contract.register(1);
            assert_eq!(Some(accounts.alice), limited_use_contract.asset_registry.get(1));

            // WHEN: I attempt to invoke the execute function
            limited_use_contract.execute(1);
            // THEN: The access attempt value is incremented by one
            assert_eq!(Some(1), limited_use_contract.usage_counter.get(accounts.alice));

            // WHEN: I attempt to invoke the execute function AGAIN
            limited_use_contract.execute(1);
            // THEN: The access attempt value is incremented by one
            assert_eq!(Some(2), limited_use_contract.usage_counter.get(accounts.alice));

            
            // WHEN: I attempt to invoke the execute function AGAIN
            limited_use_contract.execute(1);
            // THEN: The access attempt value is incremented by one
            assert_eq!(Some(2), limited_use_contract.usage_counter.get(accounts.alice));
        }

        #[ink::test]
        fn can_execute_none_value_when_asset_not_registered() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut limited_use_contract = setup_test(2, accounts.alice);
            // GIVEN: An asset class is NOT registered
            // WHEN: I attempt to invoke the execute function
            limited_use_contract.execute(1);
            // THEN: The access attempt value is incremented by one
            assert_eq!(None, limited_use_contract.usage_counter.get(accounts.alice));
        }
    }
}
