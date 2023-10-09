#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::prelude::vec::Vec;
pub use self::vickrey_auction::{
    VickreyAuction,
    VickreyAuctionRef,
};

/// a proposal represents a timelocked bid
#[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct Proposal {
    /// the ciphertext
    ciphertext: Vec<u8>,
    /// a 12-byte nonce
    nonce: Vec<u8>,
    /// the ibe ciphertext
    capsule: Vec<u8>, // a single ibe ciphertext is expected
    /// a sha256 hash of the bid amount
    commitment: Vec<u8>,
}

/// The result of an auction
#[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct AuctionResult<AccountId, Balance> {
    pub winner: AccountId,
    pub debt: Balance,
}

/// A custom type for storing revealed bids
#[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct RevealedBid<AccountId> {
    /// the bidder
    bidder: AccountId,
    /// the (supposedly) revealed amount they bid 
    bid: u128,
}

use etf_chain_extension::ext::EtfEnvironment;

#[ink::contract(env = EtfEnvironment)]
mod vickrey_auction {
    use ink::storage::Mapping;
    use scale::alloc::string::ToString;
    use sha3::Digest;
    use crate::{
        EtfEnvironment, Proposal, Vec, RevealedBid, AuctionResult};


    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        /// the origin must match the configured proxy
        NotProxy
    }

    /// the auction storage
    #[ink(storage)]
    pub struct VickreyAuction {
        /// the proxy (contract)
        proxy: AccountId,
        /// the item being auctioned
        asset_id: AssetId,
        /// a collection of proposals, one proposal per participant
        proposals: Mapping<AccountId, Proposal>,
        /// a collection of proposals marked invalid post-auction
        failed_proposals: Mapping<AccountId, Proposal>,
        /// ink mapping has no support for iteration so we need to loop over this vec to read through the proposals
        /// but maybe could do a struct instead? (acctid, vec, vec, vec)
        participants: Vec<AccountId>,
        /// the participant who won and how much they owe
        winner: Option<AuctionResult<AccountId, Balance>>,
        /// the decrypted proposals
        revealed_bids: Vec<RevealedBid<AccountId>>,
    }

    /// A proposal has been accepted
    #[ink(event)]
    pub struct BidSuccess { }

    /// A bid has been executed
    #[ink(event)]
    pub struct BidComplete {
        pub winner: bool,
    }

    /// the nft (ERC721) asset id type
    type AssetId = u32;

    impl VickreyAuction {
    
        /// Constructor that initializes a new auction
        #[ink(constructor)]
        pub fn new(
            proxy: AccountId,
            asset_id: u32,
        ) -> Self {
            let proposals = Mapping::default();
            let failed_proposals = Mapping::default();
            let participants: Vec<AccountId> = Vec::new();
            let revealed_bids: Vec<RevealedBid<AccountId>> = Vec::new();

            Self {
                proxy,
                asset_id,
                proposals,
                failed_proposals,
                participants,
                winner: None,
                revealed_bids,
            }
        }

        /// get the version of the contract
        #[ink(message)]
        pub fn get_asset_id(&self) -> AssetId {
            self.asset_id
        }

        #[ink(message)]
        pub fn get_proxy(&self) -> AccountId {
            self.proxy
        }

        #[ink(message)]
        pub fn get_winner(&self) -> Option<AuctionResult<AccountId, Balance>> {
            self.winner.clone()
        }

        /// get proposals
        #[ink(message)]
        pub fn get_proposal(
            &self, who: AccountId
        ) -> Option<Proposal> {
            self.proposals.get(who).clone()
        }

        /// get proposals
        #[ink(message)]
        pub fn get_failed_proposals(
            &self, who: AccountId
        ) -> Option<Proposal> {
            self.failed_proposals.get(who).clone()
        }

        /// get participants
        #[ink(message)]
        pub fn get_participants(&self) -> Vec<AccountId> {
            self.participants.clone()
        }

        /// get the revealed bids (empty until post-auction completion)
        #[ink(message)]
        pub fn get_revealed_bids(&self) -> Vec<RevealedBid<AccountId>> {
            self.revealed_bids.clone()
        }

        /// add a proposal to an active auction during the bidding phase
        /// a proposal is a signed, timelocked bid
        ///
        /// * `ciphertext`: The aes ciphertext
        /// * `nonce`: The aes nonce
        /// * `capsule`: The etf capsule
        /// * `commitment`: A commitment to the bid (sha256)
        ///
        #[ink(message)]
        pub fn bid(
            &mut self,
            bidder: AccountId,
            ciphertext: Vec<u8>,
            nonce: Vec<u8>,
            capsule: Vec<u8>, // single IbeCiphertext, capsule = Vec<IbeCiphertext>
            commitment: Vec<u8>,
        ) -> Result<(), Error> {
            let who = self.env().caller();
            if who != self.proxy {
                return Err(Error::NotProxy);
            }

            if !self.participants.contains(&bidder.clone()) {
                self.participants.push(bidder);
            }

            self.proposals.insert(bidder, 
                &Proposal {
                    ciphertext, 
                    nonce, 
                    capsule,
                    commitment,
                });
            Self::env().emit_event(BidSuccess{});
            Ok(())
        }

          /// complete the auction
          /// 
          /// * `revealed_bids`: A collection of (participant, revealed_bid_amount)
          ///
          #[ink(message)]
          pub fn complete(
              &mut self, 
              revealed_bids: Vec<RevealedBid<AccountId>>,
          ) -> Result<(), Error> {
            let mut highest_bid: u128 = 0;
            let mut second_highest_bid: u128 = 0;
            let mut winner: Option<AccountId> = None;
  
            for bid in revealed_bids.iter() {
                let bidder = bid.bidder;
                let b = bid.bid;
                if let Some(proposal) = self.proposals.get(bidder) {
                    let expected_hash = proposal.commitment.clone();
                    let mut hasher = sha3::Sha3_256::new();
                    let bid_bytes = b.to_string();
                    hasher.update(bid_bytes.clone());
                    let actual_hash = hasher.finalize().to_vec();

                    if expected_hash.eq(&actual_hash) {
                        self.revealed_bids.push(
                            RevealedBid {
                                bidder, 
                                bid: b
                            });
                        if b > highest_bid {
                            second_highest_bid = highest_bid;
                            highest_bid = b;
                            winner = Some(bidder);
                        }
                    } else {
                        self.failed_proposals.insert(bidder, &proposal);
                    }
                }
            }
            // Check if all participants have revealed their bids
            // if self.revealed_bids.len() == self.participants.len() {
            // Set the winner only if all bids are revealed
            if let Some(w) = winner {
                self.winner = Some(AuctionResult {
                    winner: w, 
                    debt: second_highest_bid
                });
            }
            // }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn bid_success() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut auction = VickreyAuction::new(accounts.alice, 1u32);
            let res = auction.bid(accounts.alice, vec![1], vec![2], vec![3], vec![4]);
            assert!(!res.is_err());

            let participants = auction.participants;
            assert_eq!(participants.clone().len(), 1);
            let expected_proposal = Proposal {
                ciphertext: vec![1],
                nonce: vec![2], 
                capsule: vec![3],
                commitment: vec![4],
            };
            assert_eq!(auction.proposals.get(participants[0]), Some(expected_proposal));
        }

        #[ink::test]
        fn bid_fails_when_not_proxy() {
            let accounts = 
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut auction = VickreyAuction::new(accounts.alice, 1u32);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(
                accounts.bob);

            let res = auction.bid(
                accounts.alice, 
                vec![1], 
                vec![2], 
                vec![3], 
                vec![4]
            );
            assert!(res.is_err());
            assert_eq!(res, Err(Error::NotProxy));
        }

        #[ink::test]
        fn complete_auction_success_single_participant() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut auction = VickreyAuction::new(accounts.alice, 1u32);

            let b = 4;
            let mut hasher = sha3::Sha3_256::new();
            let bid_bytes = b.to_string();
            hasher.update(bid_bytes.clone());
            let hash = hasher.finalize().to_vec();
            let res = auction.bid(accounts.alice, vec![1], vec![2], vec![3], hash);
            assert!(!res.is_err());
            let revealed_bids = vec![RevealedBid { 
                bidder: accounts.alice, 
                bid: 4,
            }];
            let res = auction.complete(revealed_bids);
            assert!(!res.is_err());
            assert_eq!(
                auction.revealed_bids.get(0).unwrap(), 
                &RevealedBid{ bidder: accounts.alice, bid: 4 });
        }

        #[ink::test]
        fn complete_auction_success_many_participants_all_valid() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut auction = VickreyAuction::new(accounts.alice, 1u32);

            let b1 = 4;
            let b1_hash = sha256(b1);

            let b2 = 6;
            let b2_hash = sha256(b2);

            let b3 = 1;
            let b3_hash = sha256(b3);

            // ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let _ = auction.bid(accounts.alice, vec![1], vec![2], vec![3], b1_hash);
            // ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let _ = auction.bid(accounts.bob, vec![1], vec![2], vec![3], b2_hash);
            // ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            let _ = auction.bid(accounts.charlie, vec![1], vec![2], vec![3], b3_hash);


            let revealed_bids = vec![
                RevealedBid { bidder: accounts.alice, bid: b1}, 
                RevealedBid { bidder: accounts.bob, bid: b2 }, 
                RevealedBid { bidder: accounts.charlie, bid: b3 }
            ];
            let res = auction.complete(revealed_bids);

            assert!(!res.is_err());
            assert_eq!(auction.revealed_bids.get(0).unwrap(), &RevealedBid{ bidder: accounts.alice, bid: 4 });
            assert_eq!(auction.revealed_bids.get(1).unwrap(), &RevealedBid{ bidder: accounts.bob, bid: 6 });
            assert_eq!(auction.revealed_bids.get(2).unwrap(), &RevealedBid{ bidder: accounts.charlie, bid: 1 });
            // bob placed the highest bid and pays alice's bid
            assert_eq!(auction.winner, Some(AuctionResult { winner: accounts.bob, debt: b1 }));
        }


        #[ink::test]
        fn complete_auction_success_many_participants_some_valid() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut auction = VickreyAuction::new(accounts.alice, 1u32);

            let b1 = 4;
            let b1_hash = sha256(b1);

            let b2 = 6;
            let b2_hash = sha256(b2);

            let b3 = 1;
            let b3_hash = sha256(b3);

            let b_invalid = 100;

            let expected_failed_proposal = Proposal {
                ciphertext: vec![1],
                nonce: vec![2], 
                capsule: vec![3],
                commitment: b3_hash.clone(),
            };

            // ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let _ = auction.bid(accounts.alice, vec![1], vec![2], vec![3], b1_hash);
            // ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let _ = auction.bid(accounts.bob, vec![1], vec![2], vec![3], b2_hash);
            // ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            let _ = auction.bid(accounts.charlie, vec![1], vec![2], vec![3], b3_hash);

            let revealed_bids = vec![
                RevealedBid { bidder: accounts.alice, bid: b1}, 
                RevealedBid { bidder: accounts.bob, bid: b2 }, 
                RevealedBid { bidder: accounts.charlie, bid: b_invalid }
            ];
            let res = auction.complete(revealed_bids);

            assert!(!res.is_err());
            assert_eq!(auction.revealed_bids.get(0).unwrap(), &RevealedBid{ bidder: accounts.alice, bid: 4 });
            assert_eq!(auction.revealed_bids.get(1).unwrap(), &RevealedBid{ bidder: accounts.bob, bid: 6 });
            // assert_eq!(auction.revealed_bids.get(2), RevealedBid{ bidder: accounts.charlie, None });
            assert_eq!(auction.failed_proposals.get(accounts.charlie), Some(expected_failed_proposal));
            // bob placed the highest bid and pays alice's bid
            assert_eq!(auction.winner, Some(AuctionResult{ winner: accounts.bob, debt: b1 }))
        }


        fn sha256(b: u128) -> Vec<u8> {
            let mut hasher = sha3::Sha3_256::new();
            let bytes = b.to_string();
            hasher.update(bytes.clone());
            hasher.finalize().to_vec()
        }
    }
}
