#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink_env::Environment;

type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;

/// the etf chain extension
#[ink::chain_extension]
pub trait ETF {
    type ErrorCode = EtfErrorCode;

    /// check if a block has been authored in the slot
    #[ink(extension = 1101, handle_status = false)]
    fn check_slot(slot_id: u32) -> bool;

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
    FailCheckSlot,
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
    use ink::prelude::vec::Vec;

    use crypto::{
        client::client::{DefaultEtfClient, EtfClient},
        ibe::fullident::BfIbe,
    };
      
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
        slot_ids: Vec<u32>,
        /// the threshold for the slot schedule
        threshold: u8,
        /// a collection of proposals, one proposal per participant
        proposals: Mapping<AccountId, (Balance, Vec<u8>, Vec<u8>, Vec<Vec<u8>>)>, // deposit, ciphertext, nonce, capsule
        /// ink mapping has no support for iteration so we need to loop over this vec to read through the proposals
        /// but maybe could do a struct instead? (acctid, vec, vec, vec)
        participants: Vec<AccountId>,
        /// a collection of participants who have 'won'
        winners: Vec<AccountId>,
        /// the decrypted proposals
        revealed_bids: Vec<Vec<u8>>,
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
            slot_ids: Vec<u32>,
            threshold: u8,
            deposit: Balance,
        ) -> Self {
            let auction_item = AuctionItem { name, asset_id, amount, verified: false };
            let proposals = Mapping::default();
            let participants: Vec<AccountId> = Vec::new();
            let winners: Vec<AccountId> = Vec::new();
            let revealed_bids: Vec<Vec<u8>> = Vec::new();
            Self {
                auction_item,
                deposit,
                slot_ids,
                threshold,
                proposals,
                participants,
                winners,
                revealed_bids,
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

        /// verifies the asset ownership and amount
        /// and then transfers the asset ownership to the contract
        #[ink(message)]
        pub fn start(&mut self) -> Result<(), Error> {
            let owner = self.env().caller();
            let contract = self.env().account_id();

            match self.env().extension().transfer_asset(
                owner, contract, 
                self.auction_item.asset_id, 
                self.auction_item.amount,
            ) {
                Ok(_) => {
                    self.auction_item.verified = true;
                    Self::env().emit_event(AuctionItemVerified{});
                    Ok(())
                },
                Err(_) => Err(Error::AssetTransferFailed),
            }
        }

        #[ink(message)]
        pub fn get_version(&self) -> Vec<u8> {
            b"0.0.1-dev".to_vec()
        }

        // add a proposal to an active auction
        // a proposal is a signed, timelocked tx that calls the 'bid' function of this contract
        #[ink(message, payable)]
        pub fn propose(&mut self, ciphertext: Vec<u8>, nonce: Vec<u8>, capsule: Vec<Vec<u8>>) -> Result<(), Error> {
            let caller = self.env().caller();

            // check min deposit
            let transferred_value = self.env().transferred_value();
            if transferred_value < self.deposit {
                return Err(Error::DepositTooLow);
            }
            
            // check deadline
            let is_past_deadline = self.env()
                .extension()
                .check_slot(self.slot_ids[self.slot_ids.len() - 1]);
            if is_past_deadline {
                return Err(Error::AuctionAlreadyComplete);
            }

            if !self.participants.contains(&caller.clone()) {
                self.participants.push(caller.clone());
            }
            self.proposals.insert(caller, &(transferred_value, ciphertext, nonce, capsule));
            Self::env().emit_event(ProposalSuccess{});
            Ok(())
        }

        #[ink(message)]
        pub fn bid(&mut self, amount: Balance) -> Result<(), Error> {
            let caller = self.env().caller();
            let is_past_deadline = self.env()
                .extension()
                .check_slot(self.slot_ids[self.slot_ids.len() - 1]);
            if is_past_deadline {
                if self.winners.contains(&caller) {
                    // payout amount to owner
                    // self.env().transfer(self.env().account_id(), amount);
                    // owner transfers nft to winner
                    Self::env().emit_event(BidComplete{
                        winner: true,
                    });
                } else {
                    // you lost, return deposit
                    let deposit = self.proposals.get(&caller).unwrap().0;
                    let _ = self.env().transfer(caller, deposit);
                    Self::env().emit_event(BidComplete{
                        winner: false,
                    });
                }
                return Ok(());
            } 

            Err(Error::AuctionInProgress)
        }

        #[ink(message)]
        pub fn complete(&mut self, pp: Vec<u8>, secrets: Vec<Vec<u8>>) -> Result<(), Error> {
            let is_past_deadline = self.env()
                .extension()
                .check_slot(self.slot_ids[self.slot_ids.len() - 1]);
            if is_past_deadline {
                // 1. ensure past deadline
                self.participants.iter().for_each(|p| {
                    self.proposals.get(&p).iter().for_each(|proposal| {
                        let signed_tx = DefaultEtfClient::<BfIbe>::decrypt(
                            pp.clone(), proposal.1.clone(), 
                            proposal.2.clone(), proposal.3.clone(), 
                            secrets.clone(),
                        ).unwrap();
                        // need to decode the tx and get the amount and use it to identify the winner
                        // 1. decode (how?!) + verify
                        // 2. check if winner
                        self.revealed_bids.push(signed_tx);
                    });
                });
                return Ok(());
            }
            Err(Error::AuctionInProgress)
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
            let slot_ids = vec![1u32, 2u32, 3u32];
            let mut auction = setup(false, false, slot_ids, 2);
            assert_eq!(auction.auction_item.verified, false);
            let _ = auction.start().ok();
            assert_eq!(auction.auction_item.verified, true);
        }

        // TODO
        // #[ink::test]
        // fn start_auction_error_on_asset_transfer_fail() {
        //     let slot_ids = vec![1u32, 2u32, 3u32];
        //     let mut auction = setup(false, true, slot_ids, 2);
        //     assert_eq!(auction.auction_item.verified, false);
        //     let res = auction.start();
        //     assert!(res.is_err());
        //     assert_eq!(auction.auction_item.verified, false);
        // }

        #[ink::test]
        fn propose_success() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));

            let slot_ids = vec![1u32, 2u32, 3u32];
            let mut auction = setup(false, false, slot_ids.clone(), 2);

            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
            let res = add_bid(slot_ids, 2, ibe_params.0, ibe_params.1, rng);
            let _ = auction.propose(res.0.clone(), res.1.clone(), res.2.clone());

            let participants = auction.participants;
            assert_eq!(participants.clone().len(), 1);
            assert_eq!(auction.proposals.get(participants[0]), 
                Some((100u128, res.0, res.1, res.2,))
            );
        }

        #[ink::test]
        fn propose_error_without_deposit() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));

            let slot_ids = vec![1u32, 2u32, 3u32];
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

            let slot_ids = vec![1u32, 2u32, 3u32];
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

            let slot_ids = vec![1u32, 2u32, 3u32];
            let mut pre_auction = setup(false, false, slot_ids.clone(), 2);

            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
            let bid = add_bid(slot_ids.clone(), 2, ibe_params.0.clone(), ibe_params.1, rng);
            let _ = pre_auction.propose(bid.0.clone(), bid.1.clone(), bid.2.clone());
            // let participants = auction.participants.clone();
            // assert_eq!(participants.clone().len(), 1);
            let mut post_auction = setup(true, false, slot_ids.clone(), 2);
            post_auction.proposals = pre_auction.proposals;
            post_auction.participants = pre_auction.participants;
            // prepare IBE slot secrets
              // setup slot ids
            let slot_ids_bytes: Vec<Vec<u8>> = slot_ids.iter().map(|s| {
                s.to_string().into_bytes().to_vec()
            }).collect();

            // in practice this would be fetched from block headers
            let ibe_slot_secrets: Vec<Vec<u8>> = ibe_extract(ibe_params.2, slot_ids_bytes).into_iter()
                .map(|(sk, _)| sk).collect::<Vec<_>>();
            // complete the auction
            let _ = post_auction.complete(ibe_params.0, ibe_slot_secrets);

            let revealed_bids = post_auction.revealed_bids;
            assert_eq!(revealed_bids.len(), 1);
            assert_eq!(revealed_bids[0], b"{I want to bid X tokens for your NFT}".to_vec());
        }

        
        #[ink::test]
        fn complete_auction_error_before_deadline() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));

            let slot_ids = vec![1u32, 2u32, 3u32];
            let mut auction = setup(false, false, slot_ids.clone(), 2);
            // in practice this would be fetched from block headers
            // complete the auction
            let r = auction.complete(ibe_params.0, vec![vec![1,2,3]]);
            assert!(r.is_err());
            assert_eq!(r, Err(Error::AuctionInProgress));
        }

        #[ink::test]
        fn bid_works_after_deadline() {
            // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
 
            let slot_ids = vec![1u32, 2u32, 3u32];
            let mut pre_auction = setup(false, false, slot_ids.clone(), 2);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
            let bid = add_bid(slot_ids.clone(), 2, ibe_params.0.clone(), ibe_params.1, rng);
            let _ = pre_auction.propose(bid.0.clone(), bid.1.clone(), bid.2.clone());
            // let participants = auction.participants.clone();
            // assert_eq!(participants.clone().len(), 1);
            let mut post_auction = setup(true, false, slot_ids.clone(), 2);
            post_auction.proposals = pre_auction.proposals;
            post_auction.participants = pre_auction.participants;

            let res = post_auction.bid(1);
            assert!(res.is_ok());
        }

        #[ink::test]
        fn bid_error_before_deadline() {
             // // we'll pretend that the blockchain is seeded with these params
             let ibe_params = test_ibe_params();
             let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
             let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
 
             let slot_ids = vec![1u32, 2u32, 3u32];
             let mut auction = setup(false, false, slot_ids.clone(), 2);
             let res = auction.bid(1);
             assert!(res.is_err());
             assert_eq!(res, Err(Error::AuctionInProgress));
        }

        fn setup(
            after_deadline: bool, 
            do_asset_transfer_fail: bool, 
            slot_ids: Vec<u32>, 
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
                    let ret: bool = false;
                    scale::Encode::encode_to(&ret, output);
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
                    let ret: bool = true;
                    scale::Encode::encode_to(&ret, output);
                    0
                }
            }
            ink_env::test::register_chain_extension(SlotsExtension);
        }

        fn add_bid(
            slots: Vec<u32>,
            threshold: u8,
            p: Vec<u8>, q: Vec<u8>, 
            rng: ChaCha20Rng
        ) -> (Vec<u8>, Vec<u8>, Vec<Vec<u8>>) {
            let mock_bid_tx = b"{I want to bid X tokens for your NFT}".to_vec();

            // setup slot ids
            let slot_ids: Vec<Vec<u8>> = slots.iter().map(|s| {
                s.to_string().into_bytes().to_vec()
            }).collect();

            let res = 
                DefaultEtfClient::<BfIbe>::encrypt(
                    p, q, &mock_bid_tx, slot_ids, threshold, rng
                ).unwrap();
            (
                res.aes_ct.ciphertext.clone(),
                res.aes_ct.nonce.clone(),
                res.etf_ct.clone(),
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
