#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink_env::Environment;
// use ark_std::vec::Vec;
use ink::prelude::vec::Vec;

type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;

/// the etf chain extension
#[ink::chain_extension]
pub trait ETF {
    type ErrorCode = EtfErrorCode;

    /// check if a block has been authored in the slot
    #[ink(extension = 1101, handle_status = false)]
    fn check_slot(slot_id: u64) -> Vec<u8>;

    // /// check if the tx bytes consist of a valid transaction
    // #[ink(extension = 1102, handle_status = true)]
    // fn check_tx(raw_tx_bytes: Vec<u8>) -> Result<Vec<u8>, EtfError>;

    /// transfer an owned asset in the assets pallet
    #[ink(extension = 2101)]
    fn transfer_asset(
        from: AccountId, 
        to: AccountId, 
        asset_id: u32, 
        amount: u128,
    ) -> Result<(), EtfError>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum EtfErrorCode {
    /// the chain ext could not check for a block in the specified slot
    FailCheckSlot,
    /// the chain ext could not verify the transaction
    FailCheckTx,
    /// the chain ext failed to transfer the asset (are you the owner?)
    FailTransferAsset,
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
            2101 => Err(Self::FailTransferAsset),
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

    type ChainExtension = ETF;
}

#[ink::contract(env = crate::CustomEnvironment)]
mod tlock_auction {
    // use super::EtfErr;
    use ink::storage::Mapping;
    use ark_serialize::CanonicalDeserialize;
    use scale::Decode;
    use crate::Vec;

    // use crypto::{
    //     client::client::{DefaultEtfClient, EtfClient},
    //     ibe::fullident::BfIbe,
    // };
      
    /// represent the asset being auctioned
    #[derive(Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct AuctionItem {
        /// the name of the auction item
        pub name: Vec<u8>,
        /// the asset id of the auction item
        pub asset_id: u32,
        /// the amount of the asset to be transferred to the winner
        pub amount: u128,
        /// indicates if the asset has been verified to exist and be owned by the deployer
        pub verified: bool,
    }

    /// a proposal represents a timelocked bid
    #[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Proposal<Balance> {
        /// the deposit transferred by the proposer
        deposit: Balance,
        /// the ciphertext
        ciphertext: Vec<u8>,
        /// a 12-byte nonce
        nonce: Vec<u8>,
        /// the ibe ciphertext
        capsule: Vec<u8>, // a single ibe ciphertext is expected
        /// a sha256 hash of the bid amount
        commitment: Vec<u8>,
    }

    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        /// the asset could not be transferred (are you the owner?)
        AssetTransferFailed,
        /// the auction has already finished
        AuctionAlreadyComplete,
        /// the auction deadline has not been reached
        AuctionInProgress,
        /// the auction requires a minimum deposit
        DepositTooLow,
        /// the current amount transferred was incorrect
        InvalidCurrencyAmountTransferred,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct TlockAuction {
        /// the item being auctioned
        auction_item: AuctionItem,
        /// the min deposit to participate (returned if honest)
        deposit: Balance,
        /// the slot schedule for this contract
        deadline: u64,
        /// the threshold for the slot schedule
        threshold: u8,
        /// a collection of proposals, one proposal per participant
        proposals: Mapping<AccountId, Proposal<Balance>>,
        /// a collection of proposals marked invalid post-auction
        failed_proposals: Mapping<AccountId, Proposal<Balance>>,
        /// ink mapping has no support for iteration so we need to loop over this vec to read through the proposals
        /// but maybe could do a struct instead? (acctid, vec, vec, vec)
        participants: Vec<AccountId>,
        /// a collection of participants who have 'won'
        winners: Vec<AccountId>,
        /// the decrypted proposals
        revealed_bids: Mapping<AccountId, u128>,
        /// track the latest error encountered in the contract (for debugging)
        err: Vec<u8>,
    }

    /// the auction item has been verified
    #[ink(event)]
    pub struct AuctionItemVerified { }

    /// A proposal has been accepted
    #[ink(event)]
    pub struct ProposalSuccess { }

    /// A bid has been executed
    #[ink(event)]
    pub struct BidComplete {
        pub winner: bool,
    }

    impl TlockAuction {

        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(
            name: Vec<u8>,
            asset_id: u32,
            amount: u128,
            deadline: u64,
            threshold: u8,
            deposit: Balance,
        ) -> Self {
            let auction_item = AuctionItem { name, asset_id, amount, verified: false };
            let proposals = Mapping::default();
            let failed_proposals = Mapping::default();
            let participants: Vec<AccountId> = Vec::new();
            let winners: Vec<AccountId> = Vec::new();
            let revealed_bids = Mapping::default();
            Self {
                auction_item,
                deposit,
                deadline,
                threshold,
                proposals,
                failed_proposals,
                participants,
                winners,
                revealed_bids,
                err: Default::default(),
            }
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(
                b"[NONAME AUCTION]".to_vec(),
                0, 0u128,
                Default::default(),
                Default::default(),
                1,
            )
        }

        /// get the version of the contract
        #[ink(message)]
        pub fn get_version(&self) -> Vec<u8> {
            b"0.0.1-dev".to_vec()
        }

        /// get the slot schedule (to encrypt messages to)
        #[ink(message)]
        pub fn get_deadline(&self) -> u64 {
            self.deadline.clone()
        }

        /// get the threshold (for creating keys)
        #[ink(message)]
        pub fn get_threshold(&self) -> u8 {
            self.threshold.clone()
        }

        /// get the minimum deposit required to participate
        #[ink(message)]
        pub fn get_deposit(&self) -> Balance {
            self.deposit.clone()
        }

        /// get proposals
        #[ink(message)]
        pub fn get_proposals(
            &self, who: AccountId
        ) -> Option<Proposal<Balance>> {
            // TODO: need to convert Vecs properly
            self.proposals.get(who).clone()
        }

        /// get proposals
        #[ink(message)]
        pub fn get_failed_proposals(
            &self, who: AccountId
        ) -> Option<Proposal<Balance>> {
            self.failed_proposals.get(who).clone()
        }

        /// get participants
        #[ink(message)]
        pub fn get_participants(&self) -> Vec<AccountId> {
            self.participants.clone()
        }

        /// get the revealed bids (empty until post-auction completion)
        #[ink(message)]
        pub fn get_revealed_bid(&self, who: AccountId) -> Option<u128> {
            self.revealed_bids.get(who).clone()
        }

        /// check if the auction item is verified to have been transferred to the contract
        /// auction winners will receive nothing if the auction is unverified when they call BID
        #[ink(message)]
        pub fn is_verified(&self) -> bool {
            self.auction_item.verified
        }

        #[ink(message)]
        pub fn get_err(&self) -> Vec<u8> {
            self.err.clone()
        }

        #[ink(message)]
        pub fn is_active(&self) -> Vec<u8> {
            self.env()
                .extension()
                .check_slot(self.deadline)
        }

        /// verifies the asset ownership and amount
        /// and then transfers the asset ownership to the contract
        #[ink(message)]
        pub fn start(&mut self) -> Result<(), Error> {
            let owner = self.env().caller();
            let contract = self.env().account_id();

            self.env()
                .extension()
                .transfer_asset(
                    owner, contract, 
                    self.auction_item.asset_id, 
                    self.auction_item.amount
                ).map(|_| {
                    self.auction_item.verified = true;
                    Self::env().emit_event(AuctionItemVerified {});
                }).map_err(|_| Error::AssetTransferFailed)
        }

        /// add a proposal to an active auction during the bidding phase
        /// a proposal is a signed, timelocked bid
        ///
        /// * `ciphertext`: The aes ciphertext
        /// * `nonce`: The aes nonce
        /// * `capsule`: The etf capsule
        /// * `commitment`: A commitment to the bid (sha256)
        ///
        #[ink(message, payable)]
        pub fn propose(
            &mut self, 
            ciphertext: Vec<u8>, 
            nonce: Vec<u8>, 
            capsule: Vec<u8>, // single IbeCiphertext, capsule = Vec<IbeCiphertext>
            commitment: Vec<u8>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            // let v = Vec::decode(&mut capsule[..]).unwrap();
            // can validate input
            // match crypto::ibe::fullident::IbeCiphertext
            //         ::deserialize_compressed(&capsule[..]) {
            //             Ok(_) => {
            //                 self.err = b"".to_vec();
            //             },
            //             Err(_) => {
            //                 self.err = b"yeah".to_vec();
            //             }
            //         }


            // check min deposit
            let transferred_value = self.env().transferred_value();
            if transferred_value < self.deposit {
                return Err(Error::DepositTooLow);
            }
            // check deadline
            let is_past_deadline = self.env()
                .extension()
                .check_slot(self.deadline);
            if is_past_deadline.eq(&[1u8]) {
                return Err(Error::AuctionAlreadyComplete);
            }

            if !self.participants.contains(&caller.clone()) {
                self.participants.push(caller.clone());
            }

            self.proposals.insert(caller, 
                &Proposal {
                    deposit: transferred_value, 
                    ciphertext, 
                    nonce, 
                    capsule,
                    commitment,
                });
            Self::env().emit_event(ProposalSuccess{});
            Ok(())
        }

          /// complete the auction
          /// 
          #[ink(message)]
          pub fn complete(
              &mut self, 
              revealed_bids: Vec<(AccountId, u128)>,
          ) -> Result<(), Error> {
            let mut highest_bid: u128 = 0;
            let mut winning_bid_index: usize = 0;
  
            // let mut sk_vec = Vec::new();
            // sk_vec.push(secret);
            // for (idx, p) in self.participants.iter().enumerate() {
            //     // TODO: handle errors - what if a proposal doesn't exist?
            //     if let Some(proposal) = self.proposals.get(&p) {

            //     }
            // }
            // let winner = self.participants[winning_bid_index];

            Ok(())
        }

        /// claim a prize or reclaim deposit, post-auction
        #[ink(message, payable)]
        pub fn claim(&mut self, amount: Balance) -> Result<(), Error> {
            let caller = self.env().caller();
            let is_past_deadline = self.env()
                .extension()
                .check_slot(self.deadline);
            if is_past_deadline.eq(&[0u8]) {
                return Err(Error::AuctionInProgress)
            }
            if self.winners.contains(&caller) {
                // 1. check if transferred_value == amount
                let transferred_value = self.env().transferred_value();
                if transferred_value != amount {
                    return Err(Error::InvalidCurrencyAmountTransferred);
                }
                // payout amount to owner
                // self.env().transfer(self.env().account_id(), amount);
                // Q: how do we do a balance transfer? another chain ext?
                // owner transfers nft to winner
                Self::env().emit_event(BidComplete {
                    winner: true,
                });
            } else {
                // you lost, return deposit
                let deposit = self.proposals.get(&caller).unwrap().deposit;
                let _ = self.env().transfer(caller, deposit);
                Self::env().emit_event(BidComplete {
                    winner: false,
                });
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crypto::testing::{test_ibe_params, ibe_extract};
        use rand_chacha::{
            rand_core::SeedableRng,
            ChaCha20Rng
        };

        #[ink::test]
        fn default_works() {
            let auction = TlockAuction::default();
            assert_eq!(auction.get_version(), b"0.0.1-dev".to_vec());
        }

        #[ink::test]
        fn start_auction_success() {
            let slot_ids = vec![1u64, 2u64, 3u64];
            let mut auction = setup(false, false, slot_ids, 2);
            assert_eq!(auction.auction_item.verified, false);
            let _ = auction.start().ok();
            assert_eq!(auction.auction_item.verified, true);
        }

        #[ink::test]
        fn propose_success() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));

            let slot_ids = vec![1u64, 2u64, 3u64];
            let mut auction = setup(false, false, slot_ids.clone(), 2);

            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
            let res = add_bid(slot_ids, 2, ibe_params.0, ibe_params.1, rng);
            let _ = auction.propose(res.0.clone(), res.1.clone(), res.2.clone());

            let participants = auction.participants;
            assert_eq!(participants.clone().len(), 1);
            let expected_proposal = Proposal {
                deposit: 100u128,
                ciphertext: res.0,
                nonce: res.1, 
                capsule: res.2,
            };
            assert_eq!(auction.proposals.get(participants[0]), Some(expected_proposal));
        }

        #[ink::test]
        fn propose_error_without_deposit() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));

            let slot_ids = vec![1u64, 2u64, 3u64];
            let mut auction = setup(false, false, slot_ids.clone(), 2);

            let bid = add_bid(slot_ids, 2, ibe_params.0, ibe_params.1, rng);
            let res = 
                auction.propose(bid.0.clone(), bid.1.clone(), bid.2.clone());
            assert!(res.is_err());
            assert_eq!(res.err(), Some(Error::DepositTooLow));
            let participants = auction.participants;
            assert_eq!(participants.clone().len(), 0);
        }

        #[ink::test]
        fn propose_error_if_past_deadline() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));

            let slot_ids = vec![1u64, 2u64, 3u64];
            let mut auction = setup(true, false, slot_ids.clone(), 2);

            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
            let bid = add_bid(slot_ids, 2, ibe_params.0, ibe_params.1, rng);
            let res = auction.propose(bid.0.clone(), bid.1.clone(), bid.2.clone());
            assert!(res.is_err());
            assert_eq!(res.err(), Some(Error::AuctionAlreadyComplete));
        }

        #[ink::test]
        fn complete_auction_after_deadline() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));

            let slot_ids = vec![1u64, 2u64, 3u64];
            let mut pre_auction = setup(false, false, slot_ids.clone(), 2);

            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
            let bid = add_bid(slot_ids.clone(), 2, ibe_params.0.clone(), ibe_params.1, rng);
            let _ = pre_auction.propose(bid.0.clone(), bid.1.clone(), bid.2.clone());
            let mut post_auction = setup(true, false, slot_ids.clone(), 2);
            post_auction.proposals = pre_auction.proposals;
            post_auction.participants = pre_auction.participants;
            // prepare IBE slot secrets
              // setup slot ids
            let slot_ids_bytes: Vec<Vec<u8>> = slot_ids.iter().map(|s| {
                s.to_string().into_bytes().to_vec()
            }).collect();

            // in practice this would be fetched from block headers
            let ibe_slot_secrets = ibe_extract(ibe_params.2, slot_ids_bytes);
            // complete the auction
            let _ = post_auction.complete(ibe_params.0, ibe_slot_secrets[0]);
            let revealed_bids = post_auction.revealed_bids;
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            assert_eq!(revealed_bids.get(accounts.alice), Some(10u128));
        }
        
        #[ink::test]
        fn complete_auction_invalid_ciphertext_or_secret_adds_to_failed_proposals() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            let slot_ids = vec![1u64, 2u64, 3u64];
            let mut auction = setup(false, false, slot_ids.clone(), 2);
            auction.participants = vec![accounts.alice];
            let mut proposals = Mapping::default();
            let proposal = Proposal { deposit: 1, ciphertext: vec![1], nonce: vec![2], capsule: vec![3u8] };
            proposals.insert(accounts.alice, &proposal.clone());
            auction.proposals = proposals;
            // in practice this would be fetched from block headers
            // complete the auction
            let r = auction.complete(ibe_params.0, vec![1u8]);
            assert_eq!(auction.failed_proposals.get(accounts.alice), Some(proposal));
        }

        // #[ink::test]
        // fn bid_works_after_deadline() {
        //     // we'll pretend that the blockchain is seeded with these params
        //     let ibe_params = test_ibe_params();
        //     let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
        //     let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
 
        //     let slot_ids = vec![1u32, 2u32, 3u32];
        //     let mut pre_auction = setup(false, false, slot_ids.clone(), 2);
        //     ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
        //     let bid = add_bid(slot_ids.clone(), 2, ibe_params.0.clone(), ibe_params.1, rng);
        //     let _ = pre_auction.propose(bid.0.clone(), bid.1.clone(), bid.2.clone());
        //     // let participants = auction.participants.clone();
        //     // assert_eq!(participants.clone().len(), 1);
        //     let mut post_auction = setup(true, false, slot_ids.clone(), 2);
        //     post_auction.proposals = pre_auction.proposals;
        //     post_auction.participants = pre_auction.participants;

        //     let res = post_auction.bid(1);
        //     assert!(res.is_ok());
        // }

        // #[ink::test]
        // fn bid_error_before_deadline() {
        //     let slot_ids = vec![1u32, 2u32, 3u32];
        //     let mut auction = setup(false, false, slot_ids.clone(), 2);
        //     let res = auction.bid(1);
        //     assert!(res.is_err());
        //     assert_eq!(res, Err(Error::AuctionInProgress));
        // }

        fn setup(
            after_deadline: bool, 
            do_asset_transfer_fail: bool, 
            slot_ids: Vec<u64>, 
            threshold: u8,
        ) -> TlockAuction {
            // setup chain extensions
            if after_deadline {
                setup_ext_slot_after_deadline();
            } else {
                setup_ext_slot_before_deadline();
            }

            if do_asset_transfer_fail {
                setup_ext_invalid_transfer();
            } else {
                setup_ext_valid_transfer();
            }
            // setup the auction contract
            TlockAuction::new(b"test1".to_vec(), 1u32, 1u128, slot_ids.clone(), threshold, 1)
        }

        fn setup_ext_valid_transfer() {
            struct TransferExtension;
            impl ink_env::test::ChainExtension for TransferExtension {
                fn func_id(&self) -> u32 {
                    2101
                }

                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    let ret: Result<(), Error> = Ok(());
                    scale::Encode::encode_to(&ret, output);
                    0
                }
            }

            ink_env::test::register_chain_extension(TransferExtension);
        }

        fn setup_ext_invalid_transfer() {
            struct TransferExtension;
            impl ink_env::test::ChainExtension for TransferExtension {
                fn func_id(&self) -> u32 {
                    2101
                }

                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    let ret: Result<(), Error> = Err(Error::AssetTransferFailed);
                    scale::Encode::encode_to(&ret, output);
                    0
                }
            }

            ink_env::test::register_chain_extension(TransferExtension);
        }


        fn setup_ext_slot_before_deadline() {
            struct SlotsExtension;
            impl ink_env::test::ChainExtension for SlotsExtension {
                fn func_id(&self) -> u32 {
                    1101
                }

                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    scale::Encode::encode_to(&vec![0u8], output);
                    0
                }
            }
            ink_env::test::register_chain_extension(SlotsExtension);
        }

        fn setup_ext_slot_after_deadline() {
            struct SlotsExtension;
            impl ink_env::test::ChainExtension for SlotsExtension {
                fn func_id(&self) -> u32 {
                    1101
                }

                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    scale::Encode::encode_to(&vec![1u8], output);
                    0
                }
            }
            ink_env::test::register_chain_extension(SlotsExtension);
        }

        fn add_bid(
            slots: Vec<u64>,
            threshold: u8,
            p: Vec<u8>, q: Vec<u8>, 
            rng: ChaCha20Rng
        ) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
            let bid = 10u128;

            // derive slot ids
            let slot_ids: Vec<Vec<u8>> = slots.iter().map(|s| {
			    s.to_string().as_bytes().to_vec()
                // s.to_string().into_bytes().to_vec()
            }).collect();

            let res = 
                DefaultEtfClient::<BfIbe>::encrypt(
                    p, q, &bid.to_le_bytes(), slot_ids, threshold, rng
                ).unwrap();

            (
                res.aes_ct.ciphertext.clone(),
                res.aes_ct.nonce.clone(),
                res.etf_ct[0].clone(),
            )
        }
    }


    // /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    // ///
    // /// When running these you need to make sure that you:
    // /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    // /// - Are running a Substrate node which contains `pallet-contracts` in the background
    // #[cfg(all(test, feature = "e2e-tests"))]
    // mod e2e_tests {
    //     /// Imports all the definitions from the outer scope so we can use them here.
    //     use super::*;

    //     /// A helper function used for calling contract messages.
    //     use ink_e2e::build_message;

    //     /// The End-to-End test `Result` type.
    //     type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    //     /// We test that we can upload and instantiate the contract using its default constructor.
    //     #[ink_e2e::test]
    //     async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
    //         // Given
    //         let constructor = SealedBidAuctionRef::default();

    //         // When
    //         let contract_account_id = client
    //             .instantiate("sealed_bid_auction", &ink_e2e::alice(), constructor, 0, None)
    //             .await
    //             .expect("instantiate failed")
    //             .account_id;

    //         // Then
    //         let get = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
    //             .call(|sealed_bid_auction| sealed_bid_auction.get());
    //         let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), false));

    //         Ok(())
    //     }

    //     /// We test that we can read and write a value from the on-chain contract contract.
    //     #[ink_e2e::test]
    //     async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
    //         // Given
    //         let constructor = SealedBidAuctionRef::new(false);
    //         let contract_account_id = client
    //             .instantiate("sealed_bid_auction", &ink_e2e::bob(), constructor, 0, None)
    //             .await
    //             .expect("instantiate failed")
    //             .account_id;

    //         let get = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
    //             .call(|sealed_bid_auction| sealed_bid_auction.get());
    //         let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), false));

    //         // When
    //         let flip = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
    //             .call(|sealed_bid_auction| sealed_bid_auction.flip());
    //         let _flip_result = client
    //             .call(&ink_e2e::bob(), flip, 0, None)
    //             .await
    //             .expect("flip failed");

    //         // Then
    //         let get = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
    //             .call(|sealed_bid_auction| sealed_bid_auction.get());
    //         let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), true));

    //         Ok(())
    //     }
    // }
}
