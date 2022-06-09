#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

/// Trait definition for ComposableAccessRules
/// 
/// # Goal
/// 
/// Provide a trait that must be implemented by rules so they can be exeucted by a rule executor
/// 
#[ink::trait_definition]
pub trait ComposableAccessRule {
    /// Execute logic to determine if the caller is authorized to 
    /// fetch data associated with the asset id
    /// 
    /// * `asset_id`: The asset id to verify access to
    /// 
    #[ink(message)]
    fn execute(&mut self, asset_id: u32, consumer: ink_env::AccountId) -> bool;
}

// #[ink::trait_definition]
// pub trait RuleExecutor {
//     #[ink(message, payable)]
//     fn execute(&mut self, asset_id: u32);
// }