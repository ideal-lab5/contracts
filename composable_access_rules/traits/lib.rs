#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

/// Allows to increment and get the current value.
#[ink::trait_definition]
pub trait ComposableAccessRule {
    /// Increments the current value of the implementer by one (1).
    #[ink(message, payable)]
    fn register(&mut self, asset_id: u32);

    /// Returns the current value of the implementer.
    #[ink(message, payable)]
    fn execute(&mut self, asset_id: u32);
}