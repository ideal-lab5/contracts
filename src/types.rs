use ink::prelude::vec::Vec;

// common types used in contracts

/// the type to track successive rounds of the game
/// e.g. {0, 1, 2, 3, ...}
pub type RoundNumber = u64;

/// the type to track the slot number associated
// with game events
pub type SlotNumber = u64;

/// a timelocked message
#[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct TlockMessage {
    /// the ciphertext
    pub ciphertext: Vec<u8>,
    /// a 12-byte nonce
    pub nonce: Vec<u8>,
    /// the ibe ciphertext
    pub capsule: Vec<u8>, // a single ibe ciphertext is expected
    // a timelock commitment
    pub commitment: Vec<u8>,
}

/// represents a new event in the game
#[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct GameEvent {
    /// a name to associate with this event
    pub name: Option<[u8;32]>,
    /// the slot in etf consensus when the event happens
    pub slot: SlotNumber,
    /// extra data that can be revealed at this slot
    /// as part of an in-game event
    pub data: Vec<TlockMessage>,
}

#[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct DecryptedData<AccountId, T> {
    pub address: AccountId,
    pub data: T,
    pub msk: [u8;32]
}

/// the event config is for configuring game event clocks (contracts) from a game master contract
#[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct EventConfig {
    /// the initial slot at which to start the event clock
    pub initial_slot: SlotNumber,
    // /// the initial block at which to start the event clock (redundant?)
    // pub initial_block: BlockNumber,
    /// the interval (in slots) between events
    pub interval: SlotNumber,
}