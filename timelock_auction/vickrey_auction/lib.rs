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
    use ink::storage::Mapping;
    use scale::alloc::string::ToString;
    use sha3::Digest;

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
            if self.revealed_bids.len() != self.participants.len() {
                return Err(Error::WaitingReveals);
            }
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
            let revealedBid = RevealedBid {
                bidder: accounts.alice,
                bid: 4,
            };
            let res = auction.save_revealed_bid(revealedBid.clone());
            assert_eq!(auction.revealed_bids[0], revealedBid);
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
            // ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let _ = auction.bid(accounts.alice);
            // ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let _ = auction.bid(accounts.bob);
            // ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
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

        // fn sha256(b: u128) -> Vec<u8> {
        //     let mut hasher = sha3::Sha3_256::new();
        //     let bytes = b.to_string();
        //     hasher.update(bytes.clone());
        //     hasher.finalize().to_vec()
        // }

        // #[ink::test]
        // fn complete_auction_after_deadline() {
        //     // // we'll pretend that the blockchain is seeded with these params
        //     let ibe_params = test_ibe_params();
        //     let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
        //     let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

        //     let deadline = 1u64;
        //     let mut pre_auction = setup(accounts.alice, false, false, deadline.clone());

        //     ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
        //     let bid = 10u128;
        //     let sealed_bid = add_bid(bid, deadline.clone(), ibe_params.0.clone(), ibe_params.1.clone(), rng);
        //     let mut hasher = sha3::Sha3_256::new();
        //     hasher.update(bid.to_string());
        //     let hash = hasher.finalize().to_vec();
        //     let _ = pre_auction.bid(
        //             sealed_bid.0.clone(), sealed_bid.1.clone(), sealed_bid.2.clone(), hash);
        //     let mut post_auction = setup(accounts.alice, true, false, deadline.clone());
        //     post_auction.proposals = pre_auction.proposals;
        //     post_auction.participants = pre_auction.participants;
        //     // prepare IBE slot secrets
        //     // setup slot ids
        //     let mut slot_ids: Vec<Vec<u8>> = Vec::new();
        //     slot_ids.push(deadline.to_string().as_bytes().to_vec());

        //     // in practice this would be fetched from block headers
        //     // let ibe_slot_secrets: Vec<Vec<u8>> = ibe_extract(ibe_params.2, slot_ids).iter()
        //     //     .map(|x| { x.0.clone() }).collect();
        //     // decrypt the bids

        //     let mut revealed_bids: Vec<(AccountId, u128)> = Vec::new();
        //     revealed_bids.push((accounts.alice, bid.clone()));
        //     // post_auction.participants.clone().iter().for_each(|participant| {
        //     //     match post_auction.proposals.get(&participant.clone()) {
        //     //         Some(proposal) => {
        //     //             let mut capsule = Vec::new();
        //     //             capsule.push(proposal.capsule);
        //     //             let bid_bytes = DefaultEtfClient::<BfIbe>::decrypt(
        //     //                 ibe_params.0.clone(),
        //     //                 proposal.ciphertext,
        //     //                 proposal.nonce,
        //     //                 capsule,
        //     //                 ibe_slot_secrets.clone(),
        //     //             ).unwrap();
        //     //             let array: [u8; 16] = bid_bytes.try_into().unwrap();
        //     //             let bid = u128::from_le_bytes(array);
        //     //             revealed_bids.push((*participant, bid));
        //     //         },
        //     //         None => {
        //     //             // todo
        //     //         }
        //     //     }
        //     // });

        //     // complete the auction
        //     let _ = post_auction.complete(revealed_bids);
        //     let revealed_bids = post_auction.revealed_bids;
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
        //     let failed_proposals = post_auction.failed_proposals;
        //     assert_eq!(failed_proposals.get(accounts.alice), None);
        //     assert_eq!(revealed_bids.get(accounts.alice), Some(10u128));
        //     assert_eq!(post_auction.winner, Some((accounts.alice, 0)));
        // }

        // #[ink::test]
        // fn complete_error_after_deadline_invalid_bid_adds_to_failed_bids() {
        //     // // we'll pretend that the blockchain is seeded with these params
        //     let ibe_params = test_ibe_params();
        //     let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
        //     let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));

        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

        //     let deadline = 1u64;
        //     let mut pre_auction = setup(accounts.alice, false, false, deadline.clone());
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

        //     ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
        //     let bid = 10u128;
        //     let sealed_bid = add_bid(bid, deadline.clone(), ibe_params.0.clone(), ibe_params.1.clone(), rng);
        //     let mut hasher = sha3::Sha3_256::new();
        //     hasher.update(bid.to_le_bytes());
        //     let hash = hasher.finalize().to_vec();

        //     // let hash = sha256(&bid.to_le_bytes()).as_slice().to_vec();
        //     let _ = pre_auction.bid(
        //             sealed_bid.0.clone(), sealed_bid.1.clone(), sealed_bid.2.clone(), hash);
        //     let mut post_auction = setup(accounts.alice, true, false, deadline.clone());
        //     post_auction.proposals = pre_auction.proposals;
        //     post_auction.participants = pre_auction.participants;
        //     // prepare IBE slot secrets
        //     // setup slot ids
        //     let mut slot_ids: Vec<Vec<u8>> = Vec::new();
        //     slot_ids.push(deadline.to_string().as_bytes().to_vec());
        //     // decrypt the bids
        //     let mut revealed_bids: Vec<(AccountId, u128)> = Vec::new();
        //     revealed_bids.push((accounts.alice, 9u128));

        //     // complete the auction
        //     let _ = post_auction.complete(revealed_bids);
        //     let failed_proposals = post_auction.failed_proposals;
        //     let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
        //     assert_eq!(failed_proposals.get(accounts.alice), post_auction.proposals.get(accounts.alice));
        //     assert_eq!(post_auction.winner, None);
        // }

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

        // fn setup(
        //     proxy: AccountId,
        //     asset_id: AssetId,
        // ) -> TlockAuction {
        //     TlockAuction::new(proxy, asset_id)
        // }

        // fn setup_ext_slot_before_deadline() {
        //     struct SlotsExtension;
        //     impl ink_env::test::ChainExtension for SlotsExtension {
        //         fn func_id(&self) -> u32 {
        //             1101
        //         }

        //         fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
        //             scale::Encode::encode_to(&vec![0u8], output);
        //             0
        //         }
        //     }
        //     ink_env::test::register_chain_extension(SlotsExtension);
        // }

        // fn setup_ext_slot_after_deadline() {
        //     struct SlotsExtension;
        //     impl ink_env::test::ChainExtension for SlotsExtension {
        //         fn func_id(&self) -> u32 {
        //             1101
        //         }

        //         fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
        //             scale::Encode::encode_to(&vec![1u8], output);
        //             0
        //         }
        //     }
        //     ink_env::test::register_chain_extension(SlotsExtension);
        // }

        // fn add_bid(
        //     bid: u128,
        //     deadline: u64,
        //     p: Vec<u8>, q: Vec<u8>,
        //     rng: ChaCha20Rng
        // ) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        //     // derive slot ids
        //     let mut slot_ids: Vec<Vec<u8>> = Vec::new();
        //     slot_ids.push(deadline.to_string().as_bytes().to_vec());

        //     let res =
        //         DefaultEtfClient::<BfIbe>::encrypt(
        //             p, q, &bid.to_le_bytes(), slot_ids, 1, rng
        //         ).unwrap();

        //     (
        //         res.aes_ct.ciphertext.clone(),
        //         res.aes_ct.nonce.clone(),
        //         res.etf_ct[0].clone(),
        //     )
        // }
    }

    // / This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    // /
    // / When running these you need to make sure that you:
    // / - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    // / - Are running a Substrate node which contains `pallet-contracts` in the background
    // #[cfg(all(test, feature = "e2e-tests"))]
    // mod e2e_tests {
    //     /// Imports all the definitions from the outer scope so we can use them here.
    //     use super::*;
    //     use erc721::Erc721Ref;
    //     use ink_e2e::build_message;
    //     /// The End-to-End test `Result` type.
    //     type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    //     /// We test that we can upload and instantiate the contract using its default constructor.
    //     #[ink_e2e::test]
    //     async fn default_works(mut client: ink_e2e::Client<C, crate::CustomEnvironment>) -> E2EResult<()> {
    //         let alice = ink_e2e::alice();
    //         let alice_bytes: [u8;32] = *alice.public_key().to_account_id().as_ref();
    //         let alice_acct = AccountId::from(alice_bytes);
    //         // first create erc721
    //         let erc721_constructor = Erc721Ref::new();
    //         let erc721_account_id = client
    //         .instantiate("erc721", &alice, erc721_constructor, 0, None)
    //         .await
    //         .expect("instantiate failed")
    //         .account_id;
    //         // Given

    //         let constructor =
    //             TlockAuctionRef::new(
    //                 alice_acct, b"test".to_vec(), erc721_account_id, 1, 100u64, 1);
    //         // When
    //         let contract_account_id = client
    //             .instantiate("tlock_auction", &alice, constructor, 0, None)
    //             .await
    //             .expect("instantiate failed")
    //             .account_id;

    //         // // Then
    //         // let get = build_message::<TlockAuctionRef>(contract_account_id.clone())
    //         //     .call(|tlock_auction| tlock_auction.is_verified());
    //         // let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
    //         // assert!(matches!(get_result.return_value(), false));

    //         Ok(())
    //     }

    //     // /// We test that we can read and write a value from the on-chain contract contract.
    //     // #[ink_e2e::test]
    //     // async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
    //     //     // Given
    //     //     let constructor = SealedBidAuctionRef::new(false);
    //     //     let contract_account_id = client
    //     //         .instantiate("sealed_bid_auction", &ink_e2e::bob(), constructor, 0, None)
    //     //         .await
    //     //         .expect("instantiate failed")
    //     //         .account_id;

    //     //     let get = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
    //     //         .call(|sealed_bid_auction| sealed_bid_auction.get());
    //     //     let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
    //     //     assert!(matches!(get_result.return_value(), false));

    //     //     // When
    //     //     let flip = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
    //     //         .call(|sealed_bid_auction| sealed_bid_auction.flip());
    //     //     let _flip_result = client
    //     //         .call(&ink_e2e::bob(), flip, 0, None)
    //     //         .await
    //     //         .expect("flip failed");

    //     //     // Then
    //     //     let get = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
    //     //         .call(|sealed_bid_auction| sealed_bid_auction.get());
    //     //     let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
    //     //     assert!(matches!(get_result.return_value(), true));

    //     //     Ok(())
    //     // }
    // }
}
