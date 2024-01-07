#![cfg_attr(not(feature = "std"), no_std)]

use crate::types::RoundNumber;

/// events for clock contract errors
#[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum ClockError {
    InitializationFailed,
    ExecutionFailed,
    ContinueFailed,
}

/// Trait definition for a clock
/// 
/// # Goal
/// 
///
/// 
#[ink::trait_definition]
pub trait ClockU8 {
    
    #[ink(message)]
    fn execute(&mut self, input: u8) -> Result<(), ClockError>;

    #[ink(message)]
    fn calculate_result(&mut self, round: RoundNumber) -> Result<(), ClockError>;
}