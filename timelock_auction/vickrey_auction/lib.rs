#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub use self::vickrey_auction::{VickreyAuction, VickreyAuctionRef};
use ink::prelude::vec::Vec;
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

use etf_contract_utils::ext::EtfEnvironment;

#[ink::contract(env = EtfEnvironment)]
mod vickrey_auction {
    use crate::{AuctionResult, EtfEnvironment, RevealedBid, Vec};

    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        /// the origin must match the configured proxy
        NotProxy,
        WaitingReveals,
        NotParticipant,
    }

    /// the auction storage
    #[ink(storage)]
    pub struct VickreyAuction {
        /// the proxy (contract)
        proxy: AccountId,
        /// the item being auctioned
        asset_id: AssetId,
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
    pub struct BidSuccess {}

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
        pub fn new(proxy: AccountId, asset_id: u32) -> Self {
            let participants: Vec<AccountId> = Vec::new();
            let revealed_bids: Vec<RevealedBid<AccountId>> = Vec::new();

            Self {
                proxy,
                asset_id,
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
        /// * `bidder`: the account bidding
        ///
        #[ink(message)]
        pub fn bid(
            &mut self,
            bidder: AccountId,
        ) -> Result<(), Error> {
            let who = self.env().caller();
            if who != self.proxy {
                return Err(Error::NotProxy);
            }

            if !self.participants.contains(&bidder.clone()) {
                self.participants.push(bidder);
            }

            Self::env().emit_event(BidSuccess {});
            Ok(())
        }

        /// Takes de incoming reveled bid and saves it in the revealed_bids array
        ///
        /// * `revealed_bid`: the revealed bid
        ///
        #[ink(message)]
        pub fn save_revealed_bid(
            &mut self,
            revealed_bid: RevealedBid<AccountId>,
        ) -> Result<(), Error> {
            let who = self.env().caller();
            if who != self.proxy {
                return Err(Error::NotProxy);
            }
            if !self.participants.contains(&revealed_bid.bidder.clone()) {
                return Err(Error::NotParticipant);
            }
            self.revealed_bids.push(RevealedBid {
                bidder: revealed_bid.bidder,
                bid: revealed_bid.bid,
            });
            Ok(())
        }

        /// Complete the auction
        /// Checks the revealed bids and determines the winner
        ///
        #[ink(message)]
        pub fn complete(&mut self) -> Result<(), Error> {
            let mut highest_bid: u128 = 0;
            let mut second_highest_bid: u128 = 0;
            let mut winner: Option<AccountId> = None;
            for bid in self.revealed_bids.iter() {
                let bidder = bid.bidder;
                let b = bid.bid;
                if b > highest_bid {
                    second_highest_bid = highest_bid;
                    highest_bid = b;
                    winner = Some(bidder);
                } else if b > second_highest_bid {
                    second_highest_bid = b;
                }
            }
            if let Some(w) = winner {
                self.winner = Some(AuctionResult {
                    winner: w,
                    debt: second_highest_bid,
                });
            }
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
            let res = auction.bid(accounts.alice);
            assert!(!res.is_err());

            let participants = auction.participants;
            assert_eq!(participants.clone().len(), 1);
        }

        #[ink::test]
        fn bid_fails_when_not_proxy() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut auction = VickreyAuction::new(accounts.alice, 1u32);
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let res = auction.bid(accounts.alice);
            assert!(res.is_err());
            assert_eq!(res, Err(Error::NotProxy));
        }

        #[ink::test]
        fn complete_auction_success_single_participant() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut auction = VickreyAuction::new(accounts.alice, 1u32);

            let res = auction.bid(accounts.alice);
            assert!(!res.is_err());
            let revealed_bid = RevealedBid {
                bidder: accounts.alice,
                bid: 4,
            };
            let _res = auction.save_revealed_bid(revealed_bid.clone());
            assert_eq!(auction.revealed_bids[0], revealed_bid);
            let res = auction.complete();
            assert!(!res.is_err());
            assert_eq!(
                auction.winner,
                Some(AuctionResult {
                    winner: accounts.alice,
                    debt: 0
                })
            )
        }

        #[ink::test]
        fn complete_auction_success_many_participants_all_valid() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut auction = VickreyAuction::new(accounts.alice, 1u32);
            let _ = auction.bid(accounts.alice);
            let _ = auction.bid(accounts.bob);
            let _ = auction.bid(accounts.charlie);
            let revealed_bids = vec![
                RevealedBid {
                    bidder: accounts.alice,
                    bid: 1,
                },
                RevealedBid {
                    bidder: accounts.bob,
                    bid: 3,
                },
                RevealedBid {
                    bidder: accounts.charlie,
                    bid: 2,
                },
            ];
            revealed_bids.iter().for_each(|bid| {
                let res = auction.save_revealed_bid(bid.clone());
                assert!(res.is_ok());
            });
            let res = auction.complete();
            assert!(res.is_ok());
            assert_eq!(auction.revealed_bids[0], revealed_bids[0].clone());
            assert_eq!(auction.revealed_bids[1], revealed_bids[1].clone());
            assert_eq!(auction.revealed_bids[2], revealed_bids[2]);
            // bob placed the highest bid and pays alice's bid
            assert_eq!(
                auction.winner,
                Some(AuctionResult {
                    winner: accounts.bob,
                    debt: 2
                })
            )
        }
    }
}
