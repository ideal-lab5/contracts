// common types used in contracts

/// the type to track successive rounds of the game
/// e.g. {0, 1, 2, 3, ...}
pub type RoundNumber = u8;

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
    ciphertext: Vec<u8>,
    /// a 12-byte nonce
    nonce: Vec<u8>,
    /// the ibe ciphertext
    capsule: Vec<u8>, // a single ibe ciphertext is expected
    // a timelock commitment
    commitment: Vec<u8>,
}

/// represents a new event in the game
#[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct GameEvent {
    /// a name to associate with this event
    name: Option<[u8;32]>,
    /// the slot in etf consensus when the event happens
    slot: SlotNumber,
    /// extra data that can be revealed at this slot
    /// as part of an in-game event
    data: Vec<TlockMessage>,
}