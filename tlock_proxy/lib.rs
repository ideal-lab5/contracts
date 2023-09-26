#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod etf_env;

// #[ink::contract]
#[ink::contract(env = crate::etf_env::EtfEnvironment)]
mod tlock_proxy {
    use ink::prelude::vec::Vec;
    use ink::ToAccountId;
    use erc721::Erc721Ref;
    use auction::SPSBAuctionRef;

    /// A custom type for storing auction's details
    #[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct AuctionDetails {
        name: Vec<u8>,
        contract_id: AccountId,
        owner: AccountId,
        deposit: Balance,
        deadline: u64,
        status: u8,
    }

    /// A custom type for representing the relationship between a bidder and an auction
    #[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Bid {
        contract_id: AccountId,
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

    impl TlockProxy {
        /// Constructor
        #[ink(constructor)]
        pub fn default(
            owner: AccountId, 
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
            name: Vec<u8>,
            asset_id: u32,
            deadline: u64,
            deposit: Balance,
        ) -> Result<()> {
            let caller = self.env().caller();
            let contract_acct_id = self.env().account_id();
            // try to mint the asset
            let mut erc721_contract: Erc721Ref =
                ink::env::call::FromAccountId::from_account_id(self.erc721);
            let _= erc721_contract.mint(asset_id).map_err(|_| Error::NFTMintFailed);

            let auction_contract =
                SPSBAuctionRef::new(contract_acct_id, asset_id)
                    .endowment(0)
                    .code_hash(self.auction_contract_code_hash)
                    .salt_bytes(name.as_slice())
                    .instantiate();
            // TODO: perform some basic validations
            let auction = AuctionDetails {
                name: name.clone(),
                contract_id: auction_contract.to_account_id(),
                owner: caller,
                deposit,
                deadline,
                status: 0,
            };
            self.auctions.push(auction);
            Ok(())
        }

        /// sends a bid to a specific auction (contract_id) if the status and dealine are valid
        /// and all conditions are satisfied
        #[ink(message, payable)]
        pub fn bid(
            &mut self,
            contract_id: AccountId,
            ciphertext: Vec<u8>,
            nonce: Vec<u8>,
            capsule: Vec<u8>,
            commitment: Vec<u8>,
        ) -> Result<()> {
            let caller = self.env().caller();
            // (AuctionDetails, AuctionContractRef)
            let mut auction_data = self.get_auction_by_contract_id(contract_id.clone())?;

            // check deadline
            let is_past_deadline = self.env()
                .extension()
                .check_slot(auction_data.0.deadline);
            if is_past_deadline.eq(&[1u8]) {
                return Err(Error::AuctionAlreadyComplete);
            }

            // check min deposit
            let transferred_value = self.env().transferred_value();
            if !transferred_value.eq(&auction_data.0.deposit) {
                return Err(Error::DepositTooLow);
            }

            auction_data.1.bid(
                ciphertext, 
                nonce, 
                capsule, 
                commitment,
            ).map(|_| {
                self.bids.push(Bid {
                    contract_id: contract_id.clone(),
                    bidder: caller,
                });
            }).map_err(|_| Error::Other);
            Ok(())
        }

        /// complete the auction
        #[ink(message)]
        pub fn complete(
            &mut self,
            contract_id: AccountId,
            revealed_bids: Vec<(AccountId, u128)>,
        ) -> Result<()> {
            let mut auction_data = self.get_auction_by_contract_id(contract_id.clone())?;
            // check deadline
            let is_past_deadline = self.env()
                .extension()
                .check_slot(auction_data.0.deadline);
            if !is_past_deadline.eq(&[1u8]) {
                return Err(Error::AuctionInProgress);
            }
            auction_data.1.complete(revealed_bids)
                .map_err(|_| Error::Other);
            Ok(())
        }

        /// claim a prize or reclaim deposit, post-auction
        #[ink(message, payable)]
        pub fn claim(&mut self, contract_id: AccountId) -> Result<()> {
            let caller = self.env().caller();
            let transferred_value = self.env().transferred_value();

            let mut auction_data = self.get_auction_by_contract_id(contract_id.clone())?;

            let is_past_deadline = self.env()
                .extension()
                .check_slot(auction_data.0.deadline);
            if !is_past_deadline.eq(&[1u8]) {
                return Err(Error::AuctionInProgress);
            }

            if let Some((winner, debt)) = auction_data.1.get_winner() {
                if winner.eq(&caller) {
                    if !transferred_value.eq(&debt) {
                        return Err(Error::InvalidCurrencyAmountTransferred);
                    }
                    // transfer NFT ownership
                    // fetch asset id from contract
                    let asset_id = auction_data.1.get_asset_id();
                    let mut erc721: Erc721Ref =
                            ink::env::call::FromAccountId::
                                from_account_id(self.erc721);
                    erc721.transfer(winner, asset_id);
                    // fetch owner from asset details
                    let owner = auction_data.0.owner;
                    // transfer tokens
                    self.env().transfer(owner, transferred_value);
                }
            }
            Ok(())
        }

        /// Simply returns current auctions.
        #[ink(message)]
        pub fn get_auctions(&self) -> Vec<u8> {
            let mut output: Vec<u8> = Vec::new();
            scale::Encode::encode_to(&self.auctions, &mut output);
            output
        }

        /// Simply returns the those auctions where auctionner is the owner.
        #[ink(message)]
        pub fn get_auctions_by_owner(&self, auctionner: AccountId) -> Vec<u8> {
            let mut output: Vec<u8> = Vec::new();
            scale::Encode::encode_to(
                &self
                    .auctions
                    .iter()
                    .filter(|x| x.owner == auctionner)
                    .cloned()
                    .collect::<Vec<AuctionDetails>>(),
                &mut output,
            );
            output
        }

        #[ink(message)]
        pub fn get_auctions_by_bidder(&self, bidder: AccountId) -> Vec<u8> {
            let mut output: Vec<u8> = Vec::new();
            scale::Encode::encode_to(
                &self
                    .auctions
                    .iter()
                    .filter(|x| {
                        self.bids
                            .iter()
                            .find(|y| y.bidder == bidder && y.contract_id == x.contract_id)
                            .is_some()
                    })
                    .cloned()
                    .collect::<Vec<AuctionDetails>>(),
                &mut output,
            );
            output
        }

        /// fetch an child auction by its account id
        ///
        /// * `contract_id`: The account id of the contract
        ///
        fn get_auction_by_contract_id(
            &self, 
            contract_id: AccountId
        ) -> Result<(AuctionDetails, SPSBAuctionRef)> {
            let auction = self
                .auctions
                .iter()
                .find(|x| x.contract_id == contract_id)
                .ok_or(Error::AuctionDoesNotExist)?;
            let auction_contract: SPSBAuctionRef =
                ink::env::call::FromAccountId::from_account_id(auction.contract_id.clone());
            Ok((auction.clone(), auction_contract))
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn default_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let tlock_proxy = TlockProxy::default(accounts.bob, Hash::from([0x01; 32]));
            let result = tlock_proxy.get_auctions();
            let auctions: Vec<AuctionDetails> = scale::Decode::decode(&mut &result[..]).unwrap();
            assert_eq!(auctions.is_empty(), true);
        }

        /// We test if the default constructor does its job.
        #[ink::test]
        fn get_by_owner_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            let auction_contract_code_hash = Hash::from([0x01; 32]);
            let nft = AccountId::from([0x01; 32]);
            let mut tlock_proxy = TlockProxy::default(accounts.bob, auction_contract_code_hash);
            assert_eq!(
                tlock_proxy.new_auction(b"NFT XXX".to_vec(), nft, 0u32, 20u64, 1),
                Ok(())
            );
            let result = tlock_proxy.get_auctions_by_owner(accounts.bob);
            let auctions: Vec<AuctionDetails> = scale::Decode::decode(&mut &result[..]).unwrap();
            assert_eq!(auctions.len() > 0, true);
            let result = tlock_proxy.get_auctions_by_owner(accounts.alice);
            let auctions: Vec<AuctionDetails> = scale::Decode::decode(&mut &result[..]).unwrap();
            assert_eq!(auctions.is_empty(), true);
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

        /// A helper function used for calling contract messages.
        use ink_e2e::build_message;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            /*  // Given
            let constructor = TlockProxyRef::default();

            // When
            let contract_account_id = client
                .instantiate("tlock_proxy", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            // Then
            let get = build_message::<TlockProxyRef>(contract_account_id.clone())
                .call(|tlock_proxy| tlock_proxy.get());
            let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false)); */

            Ok(())
        }

        /// We test that we can read and write a value from the on-chain contract contract.
        #[ink_e2e::test]
        async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            /* // Given
            let constructor = TlockProxyRef::new(false);
            let contract_account_id = client
                .instantiate("tlock_proxy", &ink_e2e::bob(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let get = build_message::<TlockProxyRef>(contract_account_id.clone())
                .call(|tlock_proxy| tlock_proxy.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), false));

            // When
            let flip = build_message::<TlockProxyRef>(contract_account_id.clone())
                .call(|tlock_proxy| tlock_proxy.flip());
            let _flip_result = client
                .call(&ink_e2e::bob(), flip, 0, None)
                .await
                .expect("flip failed");

            // Then
            let get = build_message::<TlockProxyRef>(contract_account_id.clone())
                .call(|tlock_proxy| tlock_proxy.get());
            let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
            assert!(matches!(get_result.return_value(), true)); */

            Ok(())
        }
    }
}
