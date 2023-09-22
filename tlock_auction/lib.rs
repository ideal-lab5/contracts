#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub use self::tlock_auction::{
    TlockAuction,
    TlockAuctionRef,
};

use ink_env::Environment;
use ink::prelude::vec::Vec;

// type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;

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
    use ink_env::call::{build_call, ExecutionInput, Selector};
    use ink::storage::Mapping;
    use scale::alloc::string::ToString;
    use sha3::Digest;
    use crate::{CustomEnvironment, Vec};
      
    /// represent the asset being auctioned
    #[derive(Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct AuctionItem {
        /// the name of the auction item
        pub name: Vec<u8>,
        /// the id of the NFT (ERC721)
        pub id: u32,
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
        /// this function is callable only by the auction owner
        NotAuctionOwner,
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
        /// the auction is not verified, the asset cannot be transferred
        AuctionUnverified,
    }

    /// the auction storage
    #[ink(storage)]
    pub struct TlockAuction {
        /// the erc721 contract in which the auction item exists
        erc721: AccountId,
        /// the owner of the auction
        owner: AccountId,
        /// the item being auctioned
        auction_item: AuctionItem,
        /// the min deposit to participate (returned if honest)
        deposit: Balance,
        /// the slot schedule for this contract
        deadline: u64,
        /// a collection of proposals, one proposal per participant
        proposals: Mapping<AccountId, Proposal<Balance>>,
        /// a collection of proposals marked invalid post-auction
        failed_proposals: Mapping<AccountId, Proposal<Balance>>,
        /// ink mapping has no support for iteration so we need to loop over this vec to read through the proposals
        /// but maybe could do a struct instead? (acctid, vec, vec, vec)
        participants: Vec<AccountId>,
        /// the participant who won and how much they owe
        winner: Option<(AccountId, u128)>,
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
    
        /// Constructor that initializes a new auction
        #[ink(constructor)]
        pub fn new(
            owner: AccountId,
            name: Vec<u8>,
            erc721: AccountId,
            id: u32,
            deadline: u64,
            deposit: Balance,
        ) -> Self {
            let auction_item = AuctionItem { name, id, verified: false };
            let proposals = Mapping::default();
            let failed_proposals = Mapping::default();
            let participants: Vec<AccountId> = Vec::new();
            let revealed_bids = Mapping::default();
            Self {
                erc721,
                owner,
                auction_item,
                deposit,
                deadline,
                proposals,
                failed_proposals,
                participants,
                winner: None,
                revealed_bids,
                err: Default::default(),
            }
        }

        /// get the version of the contract
        #[ink(message)]
        pub fn get_version(&self) -> Vec<u8> {
            b"0.0.1-dev".to_vec()
        }

        #[ink(message)]
        pub fn get_winner(&self) -> Option<(AccountId, u128)> {
            self.winner.clone()
        }

        /// get the slot schedule (to encrypt messages to)
        #[ink(message)]
        pub fn get_deadline(&self) -> u64 {
            self.deadline.clone()
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
            
            if !self.owner.eq(&owner) {
                return Err(Error::NotAuctionOwner);
            }

            let contract = self.env().account_id();
            // transfer ownership of the nft to the contract
            Self::approve_contract(self.erc721, contract, self.auction_item.id)
                .map(|_| {
                    Self::transfer_nft(self.erc721, owner, contract, self.auction_item.id)
                    .map(|_| {
                        self.auction_item.verified = true;
                        Self::env().emit_event(AuctionItemVerified {});
                    }).map_err(|_| Error::AssetTransferFailed)
                }).map_err(|_| Error::AssetTransferFailed)?
            
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
            // the contract can only be completed after the deadline
            // this also ensures revealed_bids can't be simply guessed
            // prior to auction close
            let is_past_deadline = self.env()
                .extension()
                .check_slot(self.deadline);
            if !is_past_deadline.eq(&[1u8]) {
                return Err(Error::AuctionInProgress);
            }

            let mut highest_bid: u128 = 0;
            let mut second_highest_bid: u128 = 0;
            let mut winning_bid_index: Option<usize> = None;
  
            let mut bids_map: Mapping<AccountId, u128> = Mapping::default();
            revealed_bids.iter().for_each(|bid| {
                bids_map.insert(bid.0, &bid.1);
            });
            
            for (idx, p) in self.participants.iter().enumerate() {
                if let Some(b) = bids_map.get(&p) {
                    // TODO: handle errors - what if a proposal doesn't exist?
                    if let Some(proposal) = self.proposals.get(&p) {
                        let expected_hash = proposal.commitment.clone();
                        let mut hasher = sha3::Sha3_256::new();
                        let bid_bytes = b.to_string();
                        hasher.update(bid_bytes.clone());
                        let actual_hash = hasher.finalize().to_vec();
                        self.err = actual_hash.clone();
                        if expected_hash.eq(&actual_hash) {
                            self.revealed_bids.insert(p, &b);
                            if b > highest_bid {
                                second_highest_bid = highest_bid;
                                highest_bid = b;
                                winning_bid_index = Some(idx);
                            }
                        } else {
                            self.failed_proposals.insert(p, &proposal);
                        }
                    }
                }
            }
            // finally set the winner
            if winning_bid_index.is_some() {
                self.winner = 
                    Some((
                        self.participants[winning_bid_index.unwrap()], 
                        second_highest_bid,
                    ));
            }

            Ok(())
        }

        /// claim a prize or reclaim deposit, post-auction
        #[ink(message, payable)]
        pub fn claim(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();
            let contract = self.env().account_id();
            let is_past_deadline = self.env()
                .extension()
                .check_slot(self.deadline);
            if is_past_deadline.eq(&[0u8]) {
                return Err(Error::AuctionInProgress)
            }
            // if the auction winner is defined...
            if self.winner.is_some() && self.winner.unwrap().0.eq(&caller) {
                // 1. check if transferred_value == amount
                let transferred_value: Balance = self.env().transferred_value();
                let debt: Balance = self.winner.expect("the winner is defined;qed").1;
                if transferred_value < debt {
                    return Err(Error::InvalidCurrencyAmountTransferred);
                }

                if !self.auction_item.verified {
                    return Err(Error::AuctionUnverified);
                }
                // winner to contract -> you paid
                // asset transfer
                // conract to owner 

                // try to transfer the asset to the winner
                return Self::transfer_nft(self.erc721, contract, caller, self.auction_item.id)
                    .map(|_| {
                        // for now... it's all free
                        // let _ = self.env().transfer(self.owner, debt);
                    }).map_err(|_| Error::AssetTransferFailed)
                // payout amount to owner
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

        /// approve the contract to transfer the NFT on your behalf
        ///
        fn approve_contract(
            erc721: AccountId,
            to: AccountId, 
            id: u32,
        ) -> Result<(), Error> {
            // execute the transfer call
            build_call::<CustomEnvironment>()
                .call(erc721)
                .gas_limit(0)
                .transferred_value(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("approve")))
                        .push_arg(to)
                        .push_arg(id)
                )
                .returns::<Result<(), Error>>()
                .invoke()
        }

        /// make a cross contract call to transfer ownership of the NFT
        fn transfer_nft(
            erc721: AccountId,
            from: AccountId, 
            to: AccountId, 
            id: u32,
        ) -> Result<(), Error> {
            // execute the transfer call
            build_call::<CustomEnvironment>()
                .call(erc721)
                .gas_limit(0)
                .transferred_value(0)
                .exec_input(
                    ExecutionInput::new(Selector::new(ink::selector_bytes!("transfer_from")))
                        .push_arg(from)
                        .push_arg(to)
                        .push_arg(id)
                )
                .returns::<Result<(), Error>>()
                .invoke()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crypto::{
            testing::{test_ibe_params},
            client::client::{DefaultEtfClient, EtfClient},
            ibe::fullident::BfIbe,
        };
        use rand_chacha::{
            rand_core::SeedableRng,
            ChaCha20Rng
        };

        // #[ink::test]
        // fn default_works() {
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
        //     let auction = TlockAuction::default(accounts.alice);
        //     assert_eq!(auction.get_version(), b"0.0.1-dev".to_vec());
        // }

        // #[ink::test]
        // fn start_auction_success_when_owner() {
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
        //     let deadline = 1u64;
        //     let mut auction = setup(accounts.alice, false, false, deadline);
        //     assert_eq!(auction.auction_item.verified, false);
        //     ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
        //     let res = auction.start();
        //     assert!(res.is_ok());
        //     // assert_eq!(auction.auction_item.verified, true);
        // }

        // #[ink::test]
        // fn start_auction_error_when_not_owner() {
        //     let deadline = 1u64;
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
        //     let mut auction = setup(accounts.alice, false, false, deadline);
        //     assert_eq!(auction.auction_item.verified, false);
        //     let account = AccountId::from([2;32]);
        //     ink::env::test::set_caller::<ink::env::DefaultEnvironment>(account);
        //     let res = auction.start();
        //     assert!(res.is_err());
        //     assert_eq!(res, Err(Error::NotAuctionOwner));
        // }

        #[ink::test]
        fn propose_success() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            let deadline = 1u64;
            let mut auction = setup(accounts.alice, false, false, deadline.clone());

            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
            let bid = 10u128;
            let res = add_bid(bid, deadline, ibe_params.0, ibe_params.1, rng);    
            let _ = auction.propose(res.0.clone(), res.1.clone(), res.2.clone(), vec![1u8]);

            let participants = auction.participants;
            assert_eq!(participants.clone().len(), 1);
            let expected_proposal = Proposal {
                deposit: 100u128,
                ciphertext: res.0,
                nonce: res.1, 
                capsule: res.2,
                commitment: vec![1u8],
            };
            assert_eq!(auction.proposals.get(participants[0]), Some(expected_proposal));
        }

        #[ink::test]
        fn propose_error_without_deposit() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            let deadline = 1u64;
            let mut auction = setup(accounts.alice, false, false, deadline.clone());

            let bid = 10u128;
            let sealed_bid = add_bid(bid, deadline, ibe_params.0, ibe_params.1, rng);    
            let res = auction.propose(sealed_bid.0.clone(), sealed_bid.1.clone(), sealed_bid.2.clone(), vec![1u8]);
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
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            let deadline = 1u64;
            let mut auction = setup(accounts.alice, true, false, deadline.clone());

            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
            let bid = add_bid(10, deadline, ibe_params.0, ibe_params.1, rng);
            let res = auction.propose(bid.0.clone(), bid.1.clone(), bid.2.clone(), vec![1u8]);
            assert!(res.is_err());
            assert_eq!(res.err(), Some(Error::AuctionAlreadyComplete));
        }

        #[ink::test]
        fn complete_auction_after_deadline() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            let deadline = 1u64;
            let mut pre_auction = setup(accounts.alice, false, false, deadline.clone());

            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
            let bid = 10u128;
            let sealed_bid = add_bid(bid, deadline.clone(), ibe_params.0.clone(), ibe_params.1.clone(), rng);
            let mut hasher = sha3::Sha3_256::new();
            hasher.update(bid.to_string());
            let hash = hasher.finalize().to_vec();
            let _ = pre_auction.propose(
                    sealed_bid.0.clone(), sealed_bid.1.clone(), sealed_bid.2.clone(), hash);
            let mut post_auction = setup(accounts.alice, true, false, deadline.clone());
            post_auction.proposals = pre_auction.proposals;
            post_auction.participants = pre_auction.participants;
            // prepare IBE slot secrets
            // setup slot ids
            let mut slot_ids: Vec<Vec<u8>> = Vec::new();
            slot_ids.push(deadline.to_string().as_bytes().to_vec());

            // in practice this would be fetched from block headers
            // let ibe_slot_secrets: Vec<Vec<u8>> = ibe_extract(ibe_params.2, slot_ids).iter()
            //     .map(|x| { x.0.clone() }).collect();
            // decrypt the bids

            let mut revealed_bids: Vec<(AccountId, u128)> = Vec::new();
            revealed_bids.push((accounts.alice, bid.clone()));
            // post_auction.participants.clone().iter().for_each(|participant| {
            //     match post_auction.proposals.get(&participant.clone()) {
            //         Some(proposal) => {
            //             let mut capsule = Vec::new();
            //             capsule.push(proposal.capsule);
            //             let bid_bytes = DefaultEtfClient::<BfIbe>::decrypt(
            //                 ibe_params.0.clone(),
            //                 proposal.ciphertext,
            //                 proposal.nonce,
            //                 capsule,
            //                 ibe_slot_secrets.clone(),
            //             ).unwrap();
            //             let array: [u8; 16] = bid_bytes.try_into().unwrap();
            //             let bid = u128::from_le_bytes(array);
            //             revealed_bids.push((*participant, bid));
            //         },
            //         None => {
            //             // todo
            //         }
            //     }
            // });
            
            // complete the auction
            let _ = post_auction.complete(revealed_bids);
            let revealed_bids = post_auction.revealed_bids;
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let failed_proposals = post_auction.failed_proposals;
            assert_eq!(failed_proposals.get(accounts.alice), None);
            assert_eq!(revealed_bids.get(accounts.alice), Some(10u128));
            assert_eq!(post_auction.winner, Some((accounts.alice, 0)));
        }
        
        #[ink::test]
        fn complete_error_after_deadline_invalid_bid_adds_to_failed_bids() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));

            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            let deadline = 1u64;
            let mut pre_auction = setup(accounts.alice, false, false, deadline.clone());
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
            let bid = 10u128;
            let sealed_bid = add_bid(bid, deadline.clone(), ibe_params.0.clone(), ibe_params.1.clone(), rng);
            let mut hasher = sha3::Sha3_256::new();
            hasher.update(bid.to_le_bytes());
            let hash = hasher.finalize().to_vec();

            // let hash = sha256(&bid.to_le_bytes()).as_slice().to_vec();
            let _ = pre_auction.propose(
                    sealed_bid.0.clone(), sealed_bid.1.clone(), sealed_bid.2.clone(), hash);
            let mut post_auction = setup(accounts.alice, true, false, deadline.clone());
            post_auction.proposals = pre_auction.proposals;
            post_auction.participants = pre_auction.participants;
            // prepare IBE slot secrets
            // setup slot ids
            let mut slot_ids: Vec<Vec<u8>> = Vec::new();
            slot_ids.push(deadline.to_string().as_bytes().to_vec());
            // decrypt the bids
            let mut revealed_bids: Vec<(AccountId, u128)> = Vec::new();
            revealed_bids.push((accounts.alice, 9u128));
            
            // complete the auction
            let _ = post_auction.complete(revealed_bids);
            let failed_proposals = post_auction.failed_proposals;
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            assert_eq!(failed_proposals.get(accounts.alice), post_auction.proposals.get(accounts.alice));
            assert_eq!(post_auction.winner, None);
        }

        // #[ink::test]
        // fn claim_error_after_deadline_when_unverified() {
        //     // // we'll pretend that the blockchain is seeded with these params
        //     let deadline = 1u64;
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
        //     let mut auction = setup(accounts.alice, true, false, deadline.clone());
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

        //     ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(10u128);
        //     auction.winner = Some((accounts.alice, 10u128));
        //     let res = auction.claim();
        //     assert!(res.is_err());
        //     assert_eq!(res, Err(Error::AuctionUnverified));
        // }

        // #[ink::test]
        // fn claim_error_after_deadline_for_auction_winner_with_too_low_currency() {
        //     // // we'll pretend that the blockchain is seeded with these params
        //     let deadline = 1u64;
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
        //     let mut auction = setup(accounts.alice, true, false, deadline.clone());
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

        //     ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1u128);
        //     auction.winner = Some((accounts.alice, 10u128));
        //     let res = auction.claim();
        //     assert!(res.is_err());
        //     assert_eq!(res, Err(Error::InvalidCurrencyAmountTransferred));
        // }

        // #[ink::test]
        // fn claim_error_before_deadline() {
        //     let deadline = 1u64;
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

        //     let mut auction = setup(accounts.alice, false, false, deadline);
        //     let res = auction.claim();
        //     assert!(res.is_err());
        //     assert_eq!(res, Err(Error::AuctionInProgress));
        // }

        fn setup(
            owner: AccountId,
            after_deadline: bool, 
            do_asset_transfer_fail: bool, 
            deadline: u64,
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
            // since we do not tests with the erc721 when executing unit tests\
            // we can just set the owner as the erc721
            TlockAuction::new(owner.clone(), b"test1".to_vec(), owner, 1u32, deadline.clone(), 1)
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
            bid: u128,
            deadline: u64,
            p: Vec<u8>, q: Vec<u8>, 
            rng: ChaCha20Rng
        ) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
            // derive slot ids
            let mut slot_ids: Vec<Vec<u8>> = Vec::new();
            slot_ids.push(deadline.to_string().as_bytes().to_vec());

            let res = 
                DefaultEtfClient::<BfIbe>::encrypt(
                    p, q, &bid.to_le_bytes(), slot_ids, 1, rng
                ).unwrap();

            (
                res.aes_ct.ciphertext.clone(),
                res.aes_ct.nonce.clone(),
                res.etf_ct[0].clone(),
            )
        }
    }

    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;
        use erc721::Erc721Ref;
        use ink_e2e::build_message;
        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, crate::CustomEnvironment>) -> E2EResult<()> {
            let alice = ink_e2e::alice();
            let alice_bytes: [u8;32] = *alice.public_key().to_account_id().as_ref();
            let alice_acct = AccountId::from(alice_bytes);
            // first create erc721
            let erc721_constructor = Erc721Ref::new();
            let erc721_account_id = client
            .instantiate("erc721", &alice, erc721_constructor, 0, None)
            .await
            .expect("instantiate failed")
            .account_id;
            // Given

            let constructor = 
                TlockAuctionRef::new(
                    alice_acct, b"test".to_vec(), erc721_account_id, 1, 100u64, 1);
            // When
            let contract_account_id = client
                .instantiate("tlock_auction", &alice, constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // // Then
            // let get = build_message::<TlockAuctionRef>(contract_account_id.clone())
            //     .call(|tlock_auction| tlock_auction.is_verified());
            // let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
            // assert!(matches!(get_result.return_value(), false));

            Ok(())
        }

        // /// We test that we can read and write a value from the on-chain contract contract.
        // #[ink_e2e::test]
        // async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        //     // Given
        //     let constructor = SealedBidAuctionRef::new(false);
        //     let contract_account_id = client
        //         .instantiate("sealed_bid_auction", &ink_e2e::bob(), constructor, 0, None)
        //         .await
        //         .expect("instantiate failed")
        //         .account_id;

        //     let get = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
        //         .call(|sealed_bid_auction| sealed_bid_auction.get());
        //     let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
        //     assert!(matches!(get_result.return_value(), false));

        //     // When
        //     let flip = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
        //         .call(|sealed_bid_auction| sealed_bid_auction.flip());
        //     let _flip_result = client
        //         .call(&ink_e2e::bob(), flip, 0, None)
        //         .await
        //         .expect("flip failed");

        //     // Then
        //     let get = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
        //         .call(|sealed_bid_auction| sealed_bid_auction.get());
        //     let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
        //     assert!(matches!(get_result.return_value(), true));

        //     Ok(())
        // }
    }
}
