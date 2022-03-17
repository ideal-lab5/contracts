#![cfg_attr(not(feature = "std"), no_std)]

use ink_env::Environment;
use ink_lang as ink;

/// Functions to interact with the Iris runtime as defined in runtime/src/lib.rs
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;

    #[ink(extension = 5, returns_result = false)]
    fn burn(caller: ink_env::AccountId, asset_id: u32, amount: u64) -> [u8; 32];
    
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IrisErr {
    FailBurn,
}

impl ink_env::chain_extension::FromStatusCode for IrisErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            5 => Err(Self::FailBurn),
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
mod limited_use_token {
    use super::IrisErr;
    use ink_storage::traits::SpreadAllocate;
    /// The LimitedUseToken storage struct
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct LimitedUseToken {
        /// Stores an asset id
        asset_id: u32,
        /// stores the number of times the asset can be accessed
        usage_limit: u32,
        /// tracks the number of times that accounts have accessed the data (or called the contract)
        access_history: ink_storage::Mapping<AccountId, u32>,
    }

    /// The asset was succesfully burned
    #[ink(event)]
    pub struct BurnSuccess { }

    /// The contract will allow the user to proceed
    #[ink(event)]
    pub struct ConditionSuccess { }

    impl LimitedUseToken {
        /// Constructor that initializes empty storage
        #[ink(constructor)]
        pub fn new(asset_id: u32, usage_limit: u32) -> Self {
            ink_lang::utils::initialize_contract(|contract: &mut Self| {
                contract.asset_id = asset_id;
                contract.usage_limit = usage_limit;
            })
        }

        /// Default constructor
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default(), Default::default())
        }

        /// TODO: need to make this part of some trait that this implements, should be common to all CARs
        #[ink(message)]
        pub fn execute(&mut self, asset_id: u32, amount: u64) {
            let caller = self.env().caller();
            // increment access history map by one
            let access_attempts = self.access_history.get(caller);
            if access_attempts.unwrap() > self.usage_limit {
                self.env().extension().burn(
                    caller, asset_id, amount,
                ).map_err(|_| {});
                self.env().emit_event(BurnSuccess { });
            } else {
                let incremented = access_attempts.unwrap() + 1;
                self.access_history.insert(&caller, &incremented);
                self.env().emit_event(ConditionSuccess { });
            }
        }

        /// get the asset id for this contract
        #[ink(message)]
        pub fn asset_id(&self) -> u32 {
            return self.asset_id;
        }

        /// get the usage limit for this contract
        #[ink(message)]
        pub fn usage_limit(&self) -> u32 {
            return self.usage_limit;
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// Imports `ink_lang` so we can use `#[ink::test]`.
        use ink_lang as ink;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn default_works() {
            let limited_use_token = LimitedUseToken::default();
            assert_eq!(limited_use_token.asset_id(), 0);
            assert_eq!(limited_use_token.usage_limit(), 0);
        }
    }
}
