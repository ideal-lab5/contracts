#![cfg_attr(not(feature = "std"), no_std)]

//!
//! This contract 
//! 
//! 

use ink_env::Environment;
use ink_lang as ink;

use ink_storage::traits::{SpreadLayout, KeyPtr};
use ink_prelude::string::String;

/// Functions to interact with the Iris runtime as defined in runtime/src/lib.rs
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;

    #[ink(extension = 0, returns_result = false)]
    fn transfer_asset(contract_account: ink_env::AccountId, consumer_account: ink_env::AccountId, asset_id: u32, asset_quantity: u64) -> [u8; 32];

    #[ink(extension = 1, returns_result = false)]
    fn mint(caller: ink_env::AccountId, target: ink_env::AccountId, asset_id: u32, amount: u64) -> [u8; 32];

    #[ink(extension = 5, returns_result = false)]
    fn lock(amount: u64) -> [u8; 32];

    #[ink(extension = 6, returns_result = false)]
    fn unlock_and_transfer(target: ink_env::AccountId) -> [u8; 32];
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IrisErr {
    FailTransferAsset,
    FailMintAssets,
    FailCreateAsset,
    FailLockCurrency,
    FailUnlockCurrency,
}

impl ink_env::chain_extension::FromStatusCode for IrisErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Err(Self::FailTransferAsset),
            1 => Err(Self::FailMintAssets),
            3 => Err(Self::FailCreateAsset),
            5 => Err(Self::FailLockCurrency),
            6 => Err(Self::FailUnlockCurrency),
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

pub struct Tag {
    index: u32,
    value: String,
}

impl SpreadLayout for Tag {
    const FOOTPRINT: u64 = 1;

    fn pull_spread(ptr: &mut KeyPtr) -> Self {
        Self {
            value: SpreadLayout::pull_spread(ptr),
        }
    }

    fn push_spread(&self, ptr: &mut KeyPtr) {
        SpreadLayout::push_spread(&self.value, ptr);
    }

    fn clear_spread(&self, ptr: &mut KeyPtr) {
        SpreadLayout::clear_spread(&self.value, ptr);
    }
}

#[ink::contract(env = crate::CustomEnvironment)]
mod generic_nft_market {
    // use ink_lang as ink;
    use ink_prelude::{
        vec::Vec,
        string::String,
    };
    use super::IrisErr;
    use ink_storage::traits::SpreadAllocate;
    use crate::Tag;

    /// Defines the storage of our contract.
    ///
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct GenericNFTMarketplace {
        /// a dataspace id that data uploaded from this contract
        /// should be associated with
        dataspace_id: u32,
        /// tags for data within the contract
        tag_registry: ink_storage::Mapping<u32, String>,
        /// tagged data
        asset_registry: ink_storage::Mapping<u32, Tag>,
    }


    impl GenericNFTMarketplace {

        /// build a new GenericNFTMarketplace
        #[ink(constructor)]
        pub fn new(dataspace_id: u32, tags: Vec<String>) -> Self {
            ink_lang::utils::initialize_contract(|contract: &mut Self| {
                contract.dataspace_id = dataspace_id;
            })
        }

        /// request to become a member of the dataspace
        #[ink(message)]
        pub fn register_asset(&mut self, asset_id: u32, tags: Vec<Tag>) {

        }

        // /// request to ingest data to the contract's specified dataspace
        // #[ink(message)]
        // pub fn ingest_data() {

        // }
    }

}
