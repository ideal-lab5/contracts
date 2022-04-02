#![cfg_attr(not(feature = "std"), no_std)]

use ink_env::Environment;
use ink_lang as ink;

/// Functions to interact with the Iris runtime as defined in runtime/src/lib.rs
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;

    #[ink(extension = 0, returns_result = false)]
    fn transfer_asset(contract_account: ink_env::AccountId, consumer_account: ink_env::AccountId, asset_id: u32, asset_quantity: u64) -> [u8; 32];

    #[ink(extension = 1, returns_result = false)]
    fn mint(caller: ink_env::AccountId, target: ink_env::AccountId, asset_id: u32, amount: u64) -> [u8; 32];

    #[ink(extension = 2, returns_result = false)]
    fn lock(amount: u64) -> [u8; 32];

    #[ink(extension = 3, returns_result = false)]
    fn unlock_and_transfer(target: ink_env::AccountId) -> [u8; 32];
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IrisErr {
    FailTransferAsset,
    FailMintAssets,
    FailLockCurrency,
    FailUnlockCurrency,
}

impl ink_env::chain_extension::FromStatusCode for IrisErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Err(Self::FailTransferAsset),
            1 => Err(Self::FailMintAssets),
            2 => Err(Self::FailLockCurrency),
            3 => Err(Self::FailUnlockCurrency),
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
    // use ink_lang as ink;
    use super::IrisErr;
    use ink_storage::traits::SpreadAllocate;

    /// Defines the storage of our contract.
    ///
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct IrisAssetExchange {
        /// maps an asset id to the owner of the token sale
        owner_registry: ink_storage::Mapping<u32, AccountId>,
        /// maps an asset id to a price
        price_registry: ink_storage::Mapping<u32, u64>,
    }

    #[ink(event)]
    pub struct ContractVersion {
        #[ink(topic)]
        version: u32,
    }

    #[ink(event)]
    pub struct AssetTransferSuccess { }

    #[ink(event)]
    pub struct NewTokenSaleSuccess { }

    #[ink(event)]
    pub struct AssetNotRegistered { }

    impl IrisAssetExchange {

        /// build a new  Iris Asset Exchange
        #[ink(constructor, payable)]
        pub fn new() -> Self {
            ink_lang::utils::initialize_contract(|_| {})
        }

        /// Default constructor
        #[ink(constructor, payable)]
        pub fn default() -> Self {
            Self::new()
        }

        /// Get the version of the contract
        #[ink(message)]
        pub fn get_version(&self) -> [u8; 32] {
            // todo: this should be a constant
            self.env().emit_event(ContractVersion{ version: 1u32 });
            return [1; 32];
        }

        /// Provide pricing for a static amount of assets.
        /// 
        /// This function mints new assets from an asset class owned by the caller 
        /// and assigns them to the contract address. It adds an item to the exchange's registry,
        /// associating the asset id with the price determined by the caller.
        /// 
        /// * `asset_id`: An asset_id associated with an owned asset class
        /// * `amount`: The amount of assets that will be minted and provisioned to the exchange
        /// * `price`: The price (in OBOL) of each token
        /// 
         #[ink(message)]
         pub fn publish_sale(&mut self, asset_id: u32, amount: u64, price: u64) {
            let caller = self.env().caller();
            self.env()
                .extension()
                .mint(
                    caller, self.env().account_id(), asset_id, amount,
                ).map_err(|_| {}).ok();
            self.owner_registry.insert(&asset_id, &caller);
            self.price_registry.insert(&asset_id, &price);
            self.env().emit_event(AssetTransferSuccess { });
         }

        /// Purchase assets from the exchange.
        /// 
        /// This function performs the following process:
        /// 1. lock price*amount tokens
        /// 2. Transfer the asset from the contract account to the caller
        /// 3. unlock the locked tokens from (1) and transfer to the owner of the asset class
        /// 
        /// * `asset_id`: The id of the owned asset class
        /// * `amount`: The amount of assets to purchase
        /// 
        #[ink(message)]
        pub fn purchase_tokens(&mut self, asset_id: u32, quantity: u64) {
            let caller = self.env().caller();
            // calculate total cost
            if let Some(price) = self.price_registry.get(&asset_id) {
                let total_cost = quantity * price;
                if let Some(owner_account) = self.owner_registry.get(&asset_id) {
                    // caller locks total_cost
                    self.env().extension().lock(total_cost).map_err(|_| {}).ok();
                    // contract grants tokens to caller
                    // TODO: Should there be some validation on owner? this call will fail if the owner is incorrect anyway
                    self.env()
                        .extension()
                        .transfer_asset(
                            self.env().account_id(), caller, asset_id, quantity, 
                        ).map_err(|_| {}).ok();
                    // caller send tokens to owner -> needs to be folded into the exrinsic itself
                    self.env().extension().unlock_and_transfer(owner_account).map_err(|_| {}).ok();
                    self.env().emit_event(AssetTransferSuccess { });
                } else {
                    self.env().emit_event(AssetNotRegistered { });    
                }
            } else {
                self.env().emit_event(AssetNotRegistered { });
            }
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;
        use ink_lang as ink;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn default_works() {
            let iris_asset_exchange = IrisAssetExchange::default();
            assert_eq!(iris_asset_exchange.get_version(), [1;32]);
        }

        #[ink::test]
        fn publish_sale_works() {
            // given
            struct MintExtension;
            impl ink_env::test::ChainExtension for MintExtension {
                fn func_id(&self) -> u32 {
                    1
                }

                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    let ret: [u8; 32] = [1; 32];
                    scale::Encode::encode_to(&ret, output);
                    0
                }
            }
            ink_env::test::register_chain_extension(MintExtension);

            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut iris_asset_exchange = IrisAssetExchange::default();
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.alice);
            // WHEN: I publish a token sale
            iris_asset_exchange.publish_sale(
                1, 1, 1, 
            );
            // THEN: it is added to the registry
            assert_eq!(iris_asset_exchange.registry.get((accounts.alice, 1)).unwrap(), 1);
        }

        #[ink::test]
        fn purchase_tokens_works() {
            struct MintExtension;
            impl ink_env::test::ChainExtension for MintExtension {
                fn func_id(&self) -> u32 {
                    1
                }

                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    let ret: [u8; 32] = [1; 32];
                    scale::Encode::encode_to(&ret, output);
                    0
                }
            }
            ink_env::test::register_chain_extension(MintExtension);

            struct TransferExtension;
            impl ink_env::test::ChainExtension for TransferExtension {
                fn func_id(&self) -> u32 {
                    0
                }

                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    let ret: [u8; 32] = [1; 32];
                    scale::Encode::encode_to(&ret, output);
                    0
                }
            }

            struct LockExtension;
            impl ink_env::test::ChainExtension for LockExtension {
                fn func_id(&self) -> u32 {
                    2
                }

                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    let ret: [u8; 32] = [1; 32];
                    scale::Encode::encode_to(&ret, output);
                    0
                }
            }

            struct UnlockExtension;
            impl ink_env::test::ChainExtension for UnlockExtension {
                fn func_id(&self) -> u32 {
                    3
                }

                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    let ret: [u8; 32] = [1; 32];
                    scale::Encode::encode_to(&ret, output);
                    0
                }
            }

            ink_env::test::register_chain_extension(TransferExtension);
            // ink_env::test::register_chain_extension(MintExtension);
            ink_env::test::register_chain_extension(LockExtension);
            ink_env::test::register_chain_extension(UnlockExtension);

            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut iris_asset_exchange = IrisAssetExchange::default();
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.alice);
            // WHEN: I publish a token sale
            iris_asset_exchange.publish_sale(
                1, 1, 1, 
            );
            // THEN: it is added to the registry
            // assert_eq!(iris_asset_exchange.registry.get((accounts.alice, 1)), 1);

            ink_env::test::set_balance::<ink_env::DefaultEnvironment>(accounts.bob, 10);
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);
            iris_asset_exchange.purchase_tokens(
                accounts.alice, 1, 1,
            );
        }
    }
}
