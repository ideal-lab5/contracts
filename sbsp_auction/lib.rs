#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub use self::spsb_auction::{
    SPSBAuction,
    SPSBAuctionRef,
};

#[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub enum Error {
    /// the origin is not authorized to call this function
    UnathorizedOrigin
}

#[ink::contract]
mod spsb_auction {
    // use ink_env::call::{build_call, ExecutionInput, Selector};
    use ink::storage::Mapping;
    use scale::{alloc::string::ToString, Encode};
    use sha3::Digest;
    use ink::prelude::vec::Vec;
    use crate::Error;

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

    /// the auction storage
    #[ink(storage)]
    pub struct SPSBAuction {
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
        winner: Option<(AccountId, u128)>,
        /// the decrypted proposals
        revealed_bids: Mapping<AccountId, u128>,
    }

    /// A proposal has been accepted
    #[ink(event)]
    pub struct ProposalSuccess { }

    /// A bid has been executed
    #[ink(event)]
    pub struct BidComplete {
        pub winner: bool,
    }

    /// the nft (ERC721) asset id type
    type AssetId = u32;

    impl SPSBAuction {
    
        /// Constructor that initializes a new auction
        #[ink(constructor)]
        pub fn new(
            proxy: AccountId,
            asset_id: u32,
        ) -> Self {
            let proposals = Mapping::default();
            let failed_proposals = Mapping::default();
            let participants: Vec<AccountId> = Vec::new();
            let revealed_bids = Mapping::default();

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
            self.asset_id.clone()
        }

        #[ink(message)]
        pub fn get_proxy(&self) -> AccountId {
            self.proxy.clone()
        }

        #[ink(message)]
        pub fn get_winner(&self) -> Option<(AccountId, u128)> {
            self.winner.clone()
        }

        /// get proposals
        #[ink(message)]
        pub fn get_proposals(
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
        pub fn get_revealed_bid(&self, who: AccountId) -> Option<u128> {
            self.revealed_bids.get(who).clone()
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
            ciphertext: Vec<u8>, 
            nonce: Vec<u8>, 
            capsule: Vec<u8>, // single IbeCiphertext, capsule = Vec<IbeCiphertext>
            commitment: Vec<u8>,
        ) -> Result<(), Error> {
            self.check_caller()?;
            let caller = self.env().caller();
            if !self.participants.contains(&caller.clone()) {
                self.participants.push(caller.clone());
            }

            self.proposals.insert(caller, 
                &Proposal {
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
          /// * `revealed_bids`: A collection of (participant, revealed_bid_amount)
          ///
          #[ink(message)]
          pub fn complete(
              &mut self, 
              revealed_bids: Vec<(AccountId, u128)>,
          ) -> Result<(), Error> {
            self.check_caller()?;
            let caller = self.env().caller();

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
            // set the winner
            if winning_bid_index.is_some() {
                self.winner = 
                    Some((
                        self.participants[winning_bid_index.unwrap()], 
                        second_highest_bid,
                    ));
            }

            Ok(())
        }

        /// check if the current caller is the authorized proxy
        fn check_caller(&self) -> Result<(), Error> {
            let caller = self.env().caller();
            let proxy = self.proxy;
            if !caller.eq(&self.proxy) {
                return Err(Error::UnathorizedOrigin);
            }
            Ok(())
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
        // TODO: only proxy + owner can call contract

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
        fn bid_success() {
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
            let _ = auction.bid(res.0.clone(), res.1.clone(), res.2.clone(), vec![1u8]);

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
        fn bid_error_without_deposit() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            let deadline = 1u64;
            let mut auction = setup(accounts.alice, false, false, deadline.clone());

            let bid = 10u128;
            let sealed_bid = add_bid(bid, deadline, ibe_params.0, ibe_params.1, rng);    
            let res = auction.bid(sealed_bid.0.clone(), sealed_bid.1.clone(), sealed_bid.2.clone(), vec![1u8]);
            assert!(res.is_err());
            assert_eq!(res.err(), Some(Error::DepositTooLow));
            let participants = auction.participants;
            assert_eq!(participants.clone().len(), 0);
        }

        #[ink::test]
        fn bid_error_if_past_deadline() {
            // // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            let deadline = 1u64;
            let mut auction = setup(accounts.alice, true, false, deadline.clone());

            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100u128);
            let bid = add_bid(10, deadline, ibe_params.0, ibe_params.1, rng);
            let res = auction.bid(bid.0.clone(), bid.1.clone(), bid.2.clone(), vec![1u8]);
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
            let _ = pre_auction.bid(
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
            let _ = pre_auction.bid(
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