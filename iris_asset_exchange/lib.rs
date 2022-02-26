#![cfg_attr(not(feature = "std"), no_std)]

use ink_env::Environment;
use ink_lang as ink;

/// This is an example of how an ink! contract may call the Substrate
/// runtime function `RandomnessCollectiveFlip::random_seed`. See the
/// file `runtime/chain-extension-example.rs` for that implementation.
///
/// Here we define the operations to interact with the Substrate runtime.
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;

    /// Note: this gives the operation a corresponding `func_id` (1101 in this case),
    /// and the chain-side chain extension will get the `func_id` to do further operations.
    #[ink(extension = 1101, returns_result = false)]
    fn transfer_assets(key: &[u8; 32]) -> [u8; 32];
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IrisErr {
    FailTransferAsset,
}

impl ink_env::chain_extension::FromStatusCode for IrisErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            1 => Err(Self::FailTransferAsset),
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
mod iris_asset_exchange {
    use ink_lang as ink;
    use super::IrisErr;

    /// Defines the storage of our contract.
    ///
    /// Here we store the random seed fetched from the chain.
    #[ink(storage)]
    pub struct IrisAssetExchange {
        // value: [u8; 32],
    }

    #[ink(event)]
    pub struct AssetTransferSuccess {
        // #[ink(topic)]
        // new: [u8; 32],
    }

    impl IrisAssetExchange {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self { }
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors may delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            // Self::new(Default::default())
            Self::new()
        }

        /// Seed a random value by passing some known argument `subject` to the runtime's
        /// random source. Then, update the current `value` stored in this contract with the
        /// new random value.
        #[ink(message)]
        pub fn transfer_asset(&self, key: [u8; 32]) -> Result<(), IrisErr> {
            let caller = self.env().caller();
            // Get the on-chain random seed
            // let new_random = self.env().extension().fetch_random(subject)?;
            // self.value = new_random;
            // Emit the `RandomUpdated` event when the random seed
            // is successfully fetched.
            // self.env().emit_event(RandomUpdated { new: new_random });
            self.env().emit_event(AssetTransferSuccess { });
            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;
        use ink_lang as ink;

        // /// We test if the default constructor does its job.
        // #[ink::test]
        // fn default_works() {
        //     let rand_extension = RandExtension::default();
        //     assert_eq!(rand_extension.get(), [0; 32]);
        // }

        // #[ink::test]
        // fn chain_extension_works() {
        //     // given
        //     struct MockedExtension;
        //     impl ink_env::test::ChainExtension for MockedExtension {
        //         /// The static function id of the chain extension.
        //         fn func_id(&self) -> u32 {
        //             1101
        //         }

        //         /// The chain extension is called with the given input.
        //         ///
        //         /// Returns an error code and may fill the `output` buffer with a
        //         /// SCALE encoded result. The error code is taken from the
        //         /// `ink_env::chain_extension::FromStatusCode` implementation for
        //         /// `RandomReadErr`.
        //         fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
        //             let ret: [u8; 32] = [1; 32];
        //             scale::Encode::encode_to(&ret, output);
        //             0
        //         }
        //     }
        //     ink_env::test::register_chain_extension(MockedExtension);
        //     let mut rand_extension = RandExtension::default();
        //     assert_eq!(rand_extension.get(), [0; 32]);

        //     // when
        //     rand_extension.update([0_u8; 32]).expect("update must work");

        //     // then
        //     assert_eq!(rand_extension.get(), [1; 32]);
        // }
    }
}
