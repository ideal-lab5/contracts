use ink_env::Environment;
use ink::prelude::vec::Vec;

/// the etf chain extension
#[ink::chain_extension]
pub trait ETF {
    type ErrorCode = EtfErrorCode;
    /// check if a block has been authored in the slot
    #[ink(extension = 1101, handle_status = false)]
    fn check_slot(slot_id: u64) -> Vec<u8>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum EtfErrorCode {
    /// the chain ext could not check for a block in the specified slot
    FailCheckSlot,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum EtfError {
  ErrorCode(EtfErrorCode),
  BufferTooSmall { required_bytes: u32 },
}

impl From<EtfErrorCode> for EtfError {
  fn from(error_code: EtfErrorCode) -> Self {
    Self::ErrorCode(error_code)
  }
}

impl From<scale::Error> for EtfError {
  fn from(_: scale::Error) -> Self {
    panic!("encountered unexpected invalid SCALE encoding")
  }
}

impl ink_env::chain_extension::FromStatusCode for EtfErrorCode {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            1101 => Err(Self::FailCheckSlot),
            _ => panic!("encountered unknown status code"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum EtfEnvironment {}

impl Environment for EtfEnvironment {
    const MAX_EVENT_TOPICS: usize =
        <ink_env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;
    type Balance = <ink_env::DefaultEnvironment as Environment>::Balance;
    type Hash = <ink_env::DefaultEnvironment as Environment>::Hash;
    type BlockNumber = <ink_env::DefaultEnvironment as Environment>::BlockNumber;
    type Timestamp = <ink_env::DefaultEnvironment as Environment>::Timestamp;

    type ChainExtension = ETF;
}
