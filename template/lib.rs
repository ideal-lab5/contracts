//! This is a template that demonstrates how to fetch the latest randomness from the drand bridge pallet.
//! This contract demonstrates:
//! 
//!     1) how to configure a contract to use the required chain extension
//!     2) how to read/write the latest randomness
//! 

#![cfg_attr(not(feature = "std"), no_std, no_main)]
use idl_contract_extension::ext::DrandEnvironment;

#[ink::contract(env = DrandEnvironment)]
mod template {
    use crate::DrandEnvironment;

    /// a type to represent the randomness fetched from the pallet (32 bytes)
    pub type Randomness = [u8;32];

    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        AnError,
    }

    impl Default for Template {
        fn default() -> Self {
            Self::new()
        }
    }

    #[ink(storage)]
    pub struct Template {
        // the latest random valued fetch by the contract
        random: Randomness,
    }

    impl Template {
        /// Constructor that initializes a new template
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                random: [0;32],
            }
        }

        /// query the stored randomness
        #[ink(message)]
        pub fn get_random(&self) -> [u8;32] {
            self.random
        }

        /// mutate the random value stored in the contract
        #[ink(message)]
        pub fn mutate_random(&mut self) -> Result<(), Error> {
            // fetch the latest randomness from the drand pallet
            let random = self.env()
                .extension()
                .random();
            self.random = random;
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn it_works() {
            let contract = Template::new();
            assert_eq!(contract.get_random(), [0u8;32]);
        }

        #[ink::test]
        fn can_mutate_randomness() {
            struct MockDrandExtension;
            impl ink::env::test::ChainExtension for MockDrandExtension {
                /// The static function id of the chain extension.
                fn ext_id(&self) -> u16 {
                    12
                }

                fn call(
                    &mut self,
                    _func_id: u16,
                    _input: &[u8],
                    output: &mut Vec<u8>,
                ) -> u32 {
                    let ret: [u8; 32] = [1; 32];
                    ink::scale::Encode::encode_to(&ret, output);
                    0
                }
            }

            ink::env::test::register_chain_extension(MockDrandExtension);
            ink::env::test::advance_block::<ink::env::DefaultEnvironment>();


            let mut contract = Template::new();

            assert_eq!(contract.get_random(), [0u8;32]);

            assert!(contract.mutate_random().is_ok());

            assert_eq!(contract.get_random(), [1u8;32]);
        }
    }
}
