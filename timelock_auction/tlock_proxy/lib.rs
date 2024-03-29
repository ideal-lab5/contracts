#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub use self::tlock_proxy::{
    TlockProxy,
    TlockProxyRef,
};

use etf_contract_utils::ext::EtfEnvironment;

#[ink::contract(env = EtfEnvironment)]
mod tlock_proxy {
    use crate::EtfEnvironment;
    use erc721::Erc721Ref;
    use ink::prelude::vec::Vec;
    use ink::ToAccountId;
    use vickrey_auction::{RevealedBid, VickreyAuctionRef};

    use sha3::{
        digest::{ExtendableOutput, Update, XofReader},
        Shake128,
    };

    /// A custom type for storing auction's details
    #[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct AuctionDetails {
        name: Vec<u8>,
        auction_id: AccountId,
        asset_id: u32,
        owner: AccountId,
        deposit: Balance,
        deadline: BlockNumber,
        published: Timestamp,
        status: u8,
        bids: u8,
    }

    /// A custom type for representing the relationship between a bidder and an auction
    #[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Bid {
        auction_id: AccountId,
        bidder: AccountId,
    }

    #[derive(Clone, PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        /// the erc721 token could not be minted
        NFTMintFailed,
        /// the erc721 token could not be transferred
        NftTransferFailed,
        /// the balance transfer failed
        BalanceTransferFailed,
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
        /// there is no auction identified by the provided id
        AuctionDoesNotExist,
        /// the auction winner has not been determined
        NoWinnerDetermined,
        /// placeholder
        Other,
    }

    /// The ERC-20 result type.
    pub type Result<T> = core::result::Result<T, Error>;

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct TlockProxy {
        /// The owner of the contract
        owner: AccountId,
        /// the erc721 contract AccountId
        erc721: AccountId,
        /// Stores references to all auctions
        auctions: Vec<AuctionDetails>,
        /// Stores references to all auctions
        bids: Vec<Bid>,
        /// The TlockAuction contract code hash
        auction_contract_code_hash: Hash,
    }

    #[ink(event)]
    pub struct AuctionCreated {
        #[ink(topic)]
        auction_id: AccountId,
    }

    impl TlockProxy {
        /// Constructor
        #[ink(constructor)]
        pub fn new(
            owner: AccountId, // needed?
            auction_contract_code_hash: Hash,
            erc721_code_hash: Hash,
        ) -> Self {
            let erc721 = Erc721Ref::new()
                .code_hash(erc721_code_hash)
                .endowment(0)
                .salt_bytes([0xde, 0xad, 0xbe, 0xef])
                .instantiate();
            Self {
                owner,
                erc721: erc721.to_account_id(),
                auctions: Vec::new(),
                bids: Vec::new(),
                auction_contract_code_hash,
            }
        }

        /// deploys a new auction contract if rules are satisfied.
        #[ink(message)]
        pub fn new_auction(
            &mut self,
            name: [u8; 48],
            deadline: BlockNumber,
            deposit: Balance,
        ) -> Result<AccountId> {
            let caller = self.env().caller();
            let contract_acct_id = self.env().account_id();
            // random asset id creation with on-chain randomness
            let mut seed = self.env().extension().secret();
            seed.clone().iter().enumerate().for_each(|(i, bit)| {
                seed[i] = *bit ^ name[i];
            });

            let mut hasher = Shake128::default();
            let bytes = seed.to_vec();
            hasher.update(&bytes.clone());
            let mut reader = hasher.finalize_xof();
            let mut asset_id_bytes = [0u8; 4];
            reader.read(&mut asset_id_bytes);
            let asset_id = u32::from_le_bytes(asset_id_bytes);

            // try to mint the asset
            let mut erc721_contract: Erc721Ref =
                ink::env::call::FromAccountId::from_account_id(self.erc721);
            erc721_contract
                .mint(asset_id)
                .map_err(|_| Error::NFTMintFailed)?;

            let auction_contract = VickreyAuctionRef::new(contract_acct_id, asset_id)
                .endowment(0)
                .code_hash(self.auction_contract_code_hash)
                .salt_bytes(name.as_slice())
                .instantiate();
            let account_id = auction_contract.to_account_id();
            let auction = AuctionDetails {
                name: name.to_vec().clone(),
                auction_id: account_id,
                asset_id,
                owner: caller,
                deposit,
                deadline,
                published: self.env().block_timestamp(),
                status: 0,
                bids: 0,
            };
            self.auctions.push(auction);
            ink::codegen::EmitEvent::<TlockProxy>::emit_event(self.env(), AuctionCreated {
                auction_id: account_id,
            });
            Ok(account_id)
        }

        /// sends a bid to a specific auction (auction_id) if the status and dealine are valid
        /// and all conditions are satisfied
        #[ink(message, payable)]
        pub fn bid(&mut self, auction_id: AccountId) -> Result<()> {
            let caller = self.env().caller();
            let mut auction_data = self.get_auction_by_auction_id(auction_id)?;
            if !self.is_deadline_future(auction_data.0.deadline) {
                return Err(Error::AuctionAlreadyComplete);
            }
            // check min deposit
            let transferred_value = self.env().transferred_value();
            if transferred_value < auction_data.0.deposit {
                return Err(Error::DepositTooLow);
            }

            auction_data
                .1
                .bid(caller)
                .map(|_| {
                    // update the number of bids
                    let mut new_auction_data = auction_data.0.clone();
                    new_auction_data.bids += 1;
                    self.auctions[auction_data.2] = new_auction_data;
                    // update the bids map
                    self.bids.push(Bid {
                        auction_id,
                        bidder: caller,
                    });
                })
                .map_err(|_| Error::Other)?;
            Ok(())
        }

        /// complete the auction
        #[ink(message)]
        pub fn complete(&mut self, auction_id: AccountId) -> Result<()> {
            let mut auction_data = self.get_auction_by_auction_id(auction_id)?;
            // check deadline
            if self.is_deadline_future(auction_data.0.deadline) {
                return Err(Error::AuctionInProgress);
            }

            auction_data.1.complete().map_err(|_| Error::Other)?;
            let mut new_auction_data = auction_data.0.clone();
            new_auction_data.status = 1;
            self.auctions[auction_data.2] = new_auction_data;
            Ok(())
        }

        /// claim a prize or reclaim deposit, post-auction
        #[ink(message, payable)]
        pub fn claim(&mut self, auction_id: AccountId) -> Result<()> {
            let caller = self.env().caller();
            let transferred_value = self.env().transferred_value();

            let auction_data = self.get_auction_by_auction_id(auction_id)?;

            if self.is_deadline_future(auction_data.0.deadline) {
                return Err(Error::AuctionInProgress);
            }

            if let Some(result) = auction_data.1.get_winner() {
                let winner = result.winner;
                let debt = result.debt;
                if winner.eq(&caller) {
                    if !transferred_value.eq(&debt) {
                        return Err(Error::InvalidCurrencyAmountTransferred);
                    }
                    // transfer NFT ownership
                    // fetch asset id from contract
                    let asset_id = auction_data.1.get_asset_id();
                    let mut erc721: Erc721Ref =
                        ink::env::call::FromAccountId::from_account_id(self.erc721);
                    erc721
                        .transfer(winner, asset_id)
                        .map_err(|_| Error::NftTransferFailed)?;
                    // fetch owner from asset details
                    let owner = auction_data.0.owner;
                    // transfer tokens
                    self.env()
                        .transfer(owner, transferred_value)
                        .map_err(|_| Error::BalanceTransferFailed)?;
                }
            }
            Ok(())
        }

        // Reveals a single bid
        #[ink(message)]
        pub fn reveal_bid(
            &mut self,
            auction_id: AccountId,
            revealed_bid: RevealedBid<AccountId>,
        ) -> Result<()> {
            let mut auction_data = self.get_auction_by_auction_id(auction_id)?;
            // check deadline
            if self.is_deadline_future(auction_data.0.deadline) {
                return Err(Error::AuctionInProgress);
            }
            auction_data
                .1
                .save_revealed_bid(revealed_bid)
                .map_err(|_| Error::Other)?;
            Ok(())
        
        }

        /// get the winner and payment owed
        /// by the winner of an auction
        #[ink(message)]
        pub fn get_winner(
            &self,
            auction_id: AccountId,
        ) -> Result<vickrey_auction::AuctionResult<AccountId, Balance>> {
            let auction_data = self.get_auction_by_auction_id(auction_id)?;
            if let Some(winner) = auction_data.1.get_winner() {
                return Ok(winner);
            }
            Err(Error::NoWinnerDetermined)
        }

        /// get the winner and payment owed
        /// by the winner of an auction
        #[ink(message)]
        pub fn get_latest_auction(
            &self,
        ) -> Result<AccountId> {
            self.auctions.last().map(|x| x.auction_id).ok_or(Error::AuctionDoesNotExist)
        }


        /// Fetch a list of all auctions
        #[ink(message)]
        pub fn get_auctions(&self) -> Result<Vec<AuctionDetails>> {
            Ok(self.auctions.clone())
        }

        /// Fetch auction details by auction contract account id
        ///
        /// * `auction_id`: The auction contract account id
        ///
        #[ink(message)]
        pub fn get_auction_details(&self, auction_id: AccountId) -> Result<AuctionDetails> {
            let auction = self.get_auction_by_auction_id(auction_id)?;
            Ok(auction.0)
        }

        #[ink(message)]
        pub fn get_auction_details_by_asset_id(&self, asset_id: u32) -> Result<AuctionDetails> {
            if let Some(auction) = self.auctions.iter().find(|x| x.asset_id == asset_id) {
                return Ok(auction.clone());
            }
            Err(Error::AuctionDoesNotExist)
        }

        /// Fetch all auctions owned by the owner
        ///
        /// * `owner`: The auction owner account id
        ///
        #[ink(message)]
        pub fn get_auctions_by_owner(&self, owner: AccountId) -> Result<Vec<AuctionDetails>> {
            Ok(self
                .auctions
                .iter()
                .filter(|x| x.owner == owner)
                .cloned()
                .collect::<Vec<AuctionDetails>>())
        }

        /// Fetch all auctions in which the bidder has placed a bid
        ///
        /// * `bidder`: The bidder account id
        ///
        #[ink(message)]
        pub fn get_auctions_by_bidder(&self, bidder: AccountId) -> Result<Vec<AuctionDetails>> {
            Ok(self
                .auctions
                .iter()
                .filter(|x| {
                    self.bids
                        .iter()
                        .any(|y| y.bidder == bidder && y.auction_id == x.auction_id)
                })
                .cloned()
                .collect::<Vec<AuctionDetails>>())
        }

        /// check if the deadline has already passed
        /// returns true if a block is present at the slot, false otherwise
        fn is_deadline_future(&self, deadline: BlockNumber) -> bool {
            let current_block: u32 = self.env().block_number();
            current_block < deadline
        }

        /// fetch an child auction by its account id
        ///
        /// * `auction_id`: The account id of the contract
        ///
        fn get_auction_by_auction_id(
            &self,
            auction_id: AccountId,
        ) -> Result<(AuctionDetails, VickreyAuctionRef, usize)> {
            let (index, auction) = self
                .auctions
                .iter()
                .enumerate()
                .find(|(_, x)| x.auction_id == auction_id)
                .ok_or(Error::AuctionDoesNotExist)?;
            let auction_contract: VickreyAuctionRef =
                ink::env::call::FromAccountId::from_account_id(auction.auction_id);
            // clippy calls out the next line, but it must be cloned (since AuctionResult does not implement Copy, because Vec does not)
            Ok((auction.clone(), auction_contract, index))
        }
    }

    /// all tests are done through e2e currently, since this contract
    /// depends on uploaded code hashes and cross contract calls
    #[cfg(test)]
    mod tests {
        // use super::*;
    }

    /// E2E Tests
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test(environment = crate::EtfEnvironment)]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let erc721_code_hash = client
                .upload("erc721", &ink_e2e::alice(), None)
                .await
                .expect("upload should be ok")
                .code_hash;

            let auction_code_hash = client
                .upload("vickrey_auction", &ink_e2e::alice(), None)
                .await
                .expect("should be ok")
                .code_hash;
            let tlock_proxy = TlockProxyRef::new(accounts.bob, auction_code_hash, erc721_code_hash);
            // When: I instantiate the contract
            let contract_account_id = client
                .instantiate("tlock_proxy", &ink_e2e::alice(), tlock_proxy, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let get_auctions =
                ink_e2e::MessageBuilder::<crate::EtfEnvironment, TlockProxyRef>::from_account_id(
                    contract_account_id,
                )
                .call(|proxy| proxy.get_auctions());

            let get_auctions_res = client
                .call(&ink_e2e::bob(), get_auctions, 0, None)
                .await
                .expect("get failed");

            assert!(matches!(
                get_auctions_res
                    .return_value()
                    .expect("should be empty")
                    .is_empty(),
                true
            ));
            Ok(())
        }

        #[ink_e2e::test(environment = crate::EtfEnvironment)]
        async fn new_auction_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let erc721_code_hash = client
                .upload("erc721", &ink_e2e::alice(), None)
                .await
                .expect("upload should be ok")
                .code_hash;

            let auction_code_hash = client
                .upload("vickrey_auction", &ink_e2e::alice(), None)
                .await
                .expect("should be ok")
                .code_hash;
            let tlock_proxy = TlockProxyRef::new(accounts.bob, auction_code_hash, erc721_code_hash);
            // When: I instantiate the contract
            let contract_account_id = client
                .instantiate("tlock_proxy", &ink_e2e::alice(), tlock_proxy, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // And: I create a new auction
            let new_auction =
                ink_e2e::MessageBuilder::<crate::EtfEnvironment, TlockProxyRef>::from_account_id(
                    contract_account_id,
                )
                .call(|proxy| proxy.new_auction(b"my_auction".to_vec(), 1u32, 1u64, 1));

            let new_auction_res = client
                .call(&ink_e2e::bob(), new_auction, 0, None)
                .await
                .expect("get failed");

            let auction_contract_id = new_auction_res.return_value().ok().unwrap();

            let get_auctions =
                ink_e2e::MessageBuilder::<crate::EtfEnvironment, TlockProxyRef>::from_account_id(
                    contract_account_id,
                )
                .call(|proxy| proxy.get_auctions());

            let get_auctions_by_id =
                ink_e2e::MessageBuilder::<crate::EtfEnvironment, TlockProxyRef>::from_account_id(
                    contract_account_id,
                )
                .call(|proxy| proxy.get_auction_details(auction_contract_id));

            let get_auctions_res = client
                .call(&ink_e2e::bob(), get_auctions, 0, None)
                .await
                .expect("get failed");

            let get_auction_by_id_res = client
                .call(&ink_e2e::bob(), get_auctions_by_id, 0, None)
                .await
                .expect("failed");

            let expected_auction_details = AuctionDetails {
                name: b"my_auction".to_vec(),
                auction_id: auction_contract_id,
                asset_id: 1u32,
                owner: accounts.alice,
                deposit: 1,
                deadline: 1u32,
                status: 0,
                bids: 0,
                published: 0,
            };
            assert!(matches!(
                get_auctions_res
                    .return_value()
                    .expect("should be non-empty")
                    .len(),
                1
            ));
            assert!(matches!(
                get_auction_by_id_res.return_value().expect("should be ok"),
                expected_auction_details
            ));
            Ok(())
        }

        #[ink_e2e::test(environment = crate::EtfEnvironment)]
        async fn bid_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let erc721_code_hash = client
                .upload("erc721", &ink_e2e::alice(), None)
                .await
                .expect("upload should be ok")
                .code_hash;

            let auction_code_hash = client
                .upload("vickrey_auction", &ink_e2e::alice(), None)
                .await
                .expect("should be ok")
                .code_hash;
            let tlock_proxy = TlockProxyRef::new(accounts.bob, auction_code_hash, erc721_code_hash);
            // When: I instantiate the contract
            let contract_account_id = client
                .instantiate("tlock_proxy", &ink_e2e::alice(), tlock_proxy, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // And: I create a new auction
            let new_auction =
                ink_e2e::MessageBuilder::<crate::EtfEnvironment, TlockProxyRef>::from_account_id(
                    contract_account_id,
                )
                .call(|proxy| {
                    proxy.new_auction(
                        b"my_auction".to_vec(),
                        1u32,
                        1000000000u64, // some slot waaaay in the future
                        1,
                    )
                });

            let new_auction_res = client
                .call(&ink_e2e::alice(), new_auction, 0, None)
                .await
                .expect("get failed");

            let auction_acct_id = new_auction_res.return_value().ok().unwrap();

            let bid_call =
                ink_e2e::MessageBuilder::<crate::EtfEnvironment, TlockProxyRef>::from_account_id(
                    contract_account_id,
                )
                .call(|p| p.bid(auction_acct_id));

            let bid_res = client
                .call(&ink_e2e::alice(), bid_call, 1, None)
                .await
                .expect("failed");

            assert!(matches!(bid_res.return_value(), Ok(())));

            let acct_bytes: [u8; 32] = *ink_e2e::alice().public_key().to_account_id().as_ref();
            let acct_id = AccountId::from(acct_bytes);
            let bid_query =
                ink_e2e::MessageBuilder::<crate::EtfEnvironment, TlockProxyRef>::from_account_id(
                    contract_account_id,
                )
                .call(|proxy| proxy.get_auctions_by_bidder(acct_id));

            let bid_query_res = client
                .call(&ink_e2e::alice(), bid_query, 0, None)
                .await
                .expect("get failed");

            let res = bid_query_res.return_value().expect("should exist");
            assert!(matches!(res.len(), 1));
            Ok(())
        }
    }
}
