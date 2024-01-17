use ink_env::Environment;

/// the etf chain extension
#[ink::chain_extension]
pub trait ETF {
    type ErrorCode = EtfErrorCode;

    #[ink(extension = 1101, handle_status = false)]
    fn secret() -> [u8;48];
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum EtfErrorCode {
    /// the provided slot has no corresponding secret
    InvalidSlot,
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
            1101 => Err(Self::InvalidSlot),
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
