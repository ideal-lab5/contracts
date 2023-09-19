#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod tlock_proxy {
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    /// A custom type that we can use in our contract storage
    #[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct AuctionDetails {
        name: Vec<u8>,
        contract_id: Vec<u8>,
        owner: AccountId,
        bidders: Vec<AccountId>,
        threshold: u8,
        deadline: u64,
        status: u8,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct TlockProxy {
        /// The owner of the contract
        owner: AccountId,
        /// Stores references to all auctions
        auctions: Vec<AuctionDetails>,
    }

    impl TlockProxy {
        /// Constructor
        #[ink(constructor)]
        pub fn default(owner: AccountId) -> Self {
            Self {
                owner,
                auctions: Vec::new(),
            }
        }

        /// A message that can be called on instantiated contracts.
        /// This one flips the value of the stored `bool` from `true`
        /// to `false` and vice versa.
        #[ink(message)]
        pub fn new_auction(
            &mut self,
            name: Vec<u8>,
            asset_id: u32,
            amount: u128,
            threshold: u8,
            deadline: u64,
            deposit: Balance,
        ) {
            let caller = self.env().caller();
            // TODO: deploy a new tlock_auction contract
            let auction = AuctionDetails {
                name,
                contract_id: Vec::new(),
                owner: caller,
                bidders: Vec::new(),
                threshold,
                deadline,
                status: 0,
            };
            self.auctions.push(auction);
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
            let owner = AccountId::from([0x01; 32]);
            let tlock_proxy = TlockProxy::default(owner);
            let result = tlock_proxy.get_auctions();
            let auctions: Vec<AuctionDetails> = scale::Decode::decode(&mut &result[..]).unwrap();
            assert_eq!(auctions.is_empty(), true);
        }

        /// We test if the default constructor does its job.
        #[ink::test]
        fn get_by_owner_works() {
            let auctionner1 = AccountId::from([0x01; 32]);
            let auctionner2 = AccountId::from([0x02; 32]);
            let mut tlock_proxy = TlockProxy::default(auctionner1);
            tlock_proxy.new_auction(b"NFT XXX".to_vec(), 0u32, 0u128, 10u8, 20u64, 1);
            let result = tlock_proxy.get_auctions_by_owner(auctionner1);
            let auctions: Vec<AuctionDetails> = scale::Decode::decode(&mut &result[..]).unwrap();
            assert_eq!(auctions.len() > 0, true);
            let result = tlock_proxy.get_auctions_by_owner(auctionner2);
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
