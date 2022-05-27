#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::trait_definition]
pub trait ComposableAccessRule {
    /// Register the asset id with the contract
    /// 
    /// * `asset_id`: The asset id to register
    /// 
    #[ink(message, payable)]
    fn register(&mut self, asset_id: u32);

    /// Execute logic to determine if the caller is authorized to 
    /// fetch data associated with the asset id
    /// 
    /// * `asset_id`: The asset id to verify access to
    /// 
    #[ink(message, payable)]
    fn execute(&mut self, asset_id: u32);
}