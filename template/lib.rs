#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::prelude::vec::Vec;
use etf_contract_utils::ext::EtfEnvironment;

#[ink::contract(env = EtfEnvironment)]
mod vickrey_auction {
    use ink::storage::Mapping;
    use scale::alloc::string::ToString;
    use sha3::Digest;
    use crate::EtfEnvironment;


    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        /// the origin must match the configured proxy
        AnError,
    }

    /// the auction storage
    #[ink(storage)]
    pub struct Template {
        value: u8,
    }

    impl Template {
    
        /// Constructor that initializes a new auction
        #[ink(constructor)]
        pub fn new(value: u8) -> Self {
            Self {
                value
            }
        }

        #[ink(message)]
        pub fn get_value(&self) -> u8 {
            self.value.clone()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn it_works() {
            let mut contract = Template::new(1u8);
            assert_eq!(contract.get_value(), 1u8);
        }
    }
}
