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
use ink_lang as ink;
use ink_storage::traits::{
    SpreadLayout, 
    PackedLayout, 
    SpreadAllocate
};

pub use self::limited_use_rule::{
    LimitedUseRule,
    LimitedUseRuleRef,
};

#[derive(
    scale::Encode, scale::Decode, PartialEq, Debug, Clone, Copy, SpreadLayout, PackedLayout, SpreadAllocate,
)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
)]
struct Usage {
    asset_id: u32,
    access_attempts: u32,
}

#[ink::contract]
mod limited_use_rule {
    use ink_storage::traits::SpreadAllocate;
    use traits::ComposableAccessRule;
    use crate::Usage;
    use ink_prelude::vec::Vec;

    #[ink(event)]
    pub struct LimitExceeded{}

    #[ink(event)]
    pub struct AccessAllowed{}

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct LimitedUseRule {
        limit: u32,
        usage_counter: ink_storage::Mapping<AccountId, Vec<Usage>>,
    }

    impl LimitedUseRule {
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

    impl ComposableAccessRule for LimitedUseRule {

        /// check if the number of times a caller has attempted access to the asset 
        /// exceeds the pre-defined limit amount
        /// 
        /// * `asset_id`: The asset to which access is attempted
        /// 
        #[ink(message, payable)]
        fn execute(&mut self, asset_id: u32, consumer: ink_env::AccountId) -> bool {
            if let Some(mut usage_attempts) = self.usage_counter.get(&consumer) {
                let index = usage_attempts.iter().position(|x| x.asset_id == asset_id).unwrap();
                let u = usage_attempts[index];
                if u.access_attempts < self.limit {
                    usage_attempts.remove(index);
                    let new_usage = Usage{
                        asset_id: asset_id,
                        access_attempts: u.access_attempts + 1,
                    };
                    let mut usage_vec = usage_attempts;
                    usage_vec.push(new_usage);
                    self.usage_counter.insert(&consumer, &usage_vec);
                    self.env().emit_event(AccessAllowed{});
                    return true;
                } else {
                    self.env().emit_event(LimitExceeded{});
                    return false;
                }
            } else {
                let mut new_usage_vec = Vec::new();
                new_usage_vec.push(Usage{
                    asset_id: asset_id,
                    access_attempts: 1,
                });
                self.usage_counter.insert(&consumer, &new_usage_vec);
                self.env().emit_event(AccessAllowed{});
                return true;
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
            let limited_use_contract = LimitedUseRule::new(limit);
            assert_eq!(limit, limited_use_contract.get_limit());
        }

        /**
         * Tests for the `register` function
         */

        fn setup_test(limit: u32, default_account: ink_env::AccountId) -> LimitedUseRule {
            let limited_use_contract = LimitedUseRule::new(limit);
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(default_account);
            limited_use_contract
        }
        
        #[ink::test]
        fn can_execute_and_increment_on_first_access() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut limited_use_contract = setup_test(2, accounts.alice);

            // WHEN: I attempt to invoke the execute function
            limited_use_contract.execute(1, accounts.alice);
            // THEN: there is a usage attempt added 
            let usage_tracker = limited_use_contract.usage_counter.get(accounts.alice).unwrap();
            let usage_len = usage_tracker.len();
            assert_eq!(1, usage_len);
            // AND: The only entry contains my asset id as accessed a single time
            assert_eq!(1, usage_tracker[0].asset_id);
            assert_eq!(1, usage_tracker[0].access_attempts);
        }

        #[ink::test]
        fn can_execute_and_increment_on_second_access() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut limited_use_contract = setup_test(2, accounts.alice);

            // WHEN: I attempt to invoke the execute function
            limited_use_contract.execute(1, accounts.alice);
            // THEN: The access attempt value is incremented by one
            let usage_tracker_1 = limited_use_contract.usage_counter.get(accounts.alice).unwrap();
            assert_eq!(1, usage_tracker_1[0].access_attempts);

            // WHEN: I attempt to invoke the execute function AGAIN
            limited_use_contract.execute(1, accounts.alice);
            // THEN: The access attempt value is incremented by one
            let usage_tracker_2 = limited_use_contract.usage_counter.get(accounts.alice).unwrap();
            assert_eq!(2, usage_tracker_2[0].access_attempts);
        }

        #[ink::test]
        fn can_execute_and_not_increment_when_limit_exceeded() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut limited_use_contract = setup_test(2, accounts.alice);

            // WHEN: I attempt to invoke the execute function
            limited_use_contract.execute(1, accounts.alice);
            // THEN: The access attempt value is incremented by one
            let usage_tracker_1 = limited_use_contract.usage_counter.get(accounts.alice).unwrap();
            assert_eq!(1, usage_tracker_1[0].access_attempts);
            // WHEN: I attempt to invoke the execute function AGAIN
            limited_use_contract.execute(1, accounts.alice);
            // THEN: The access attempt value is incremented by one
            let usage_tracker_2 = limited_use_contract.usage_counter.get(accounts.alice).unwrap();
            assert_eq!(2, usage_tracker_2[0].access_attempts);
            // WHEN: I attempt to invoke the execute function AGAIN
            limited_use_contract.execute(1, accounts.alice);
            // THEN: The access attempt value is incremented by one
            let usage_tracker_3 = limited_use_contract.usage_counter.get(accounts.alice).unwrap();
            assert_eq!(2, usage_tracker_3[0].access_attempts);
        }

    }
}
