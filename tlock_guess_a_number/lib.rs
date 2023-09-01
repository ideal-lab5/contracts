#![cfg_attr(not(feature = "std"), no_std, no_main)]
//use tlock;

#[ink::contract]
mod sealed_bid_auction {
    use ink::storage::Mapping;
    use ink::prelude::{vec, vec::Vec};

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct TlockGuessANumber {
        /// the final slot number in the slot schedule
        slots: Vec<u32>,
        /// the aes pubkey
        public_key: [u8;32],
        /// the aes nonce
        nonce: Vec<u8>,
        /// the (IBE) encrypted shares of the aes msk
        encrypted_shares: Vec<u8>,
        messages: Mapping<AccountId, Vec<u8>>,
        /// ink mapping has no support for iteration...
        participants: Vec<AccountId>,
        /// write the revealed messages
        revealed_messages: Vec<Vec<u8>>,
    }

    impl TlockGuessANumber {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(
            slots: Vec<u32>,
            public_key: [u8;32],
            nonce: Vec<u8>,
            encrypted_shares: Vec<u8>,
        ) -> Self {
            let messages = Mapping::default();
            let participants: Vec<AccountId> = Vec::new();
            let revealed_messages: Vec<Vec<u8>> = Vec::new();
            Self {
                slots, 
                public_key,
                nonce, 
                encrypted_shares,
                messages,
                participants,
                revealed_messages,
            }
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            )
        }

        // add your guess
        #[ink(message)]
        pub fn publish(&mut self, msg: Vec<u8>) {
            let caller = self.env().caller();
            // 1. need to get current slot/block and ensure less than deadline `get_latest_slot()`
            // 2. other checks? [no duplicates, block_list, allow_list]
            // 3. add tlocked tx: [u8; 496] and storage_proof to storage
            if !self.participants.contains(&caller.clone()) {
                self.participants.push(caller.clone());
            }
            self.messages.insert(caller, &msg);
        }

        #[ink(message)]
        pub fn reveal(&mut self, msk: [u8;32]) {
            // 1. ensure past deadline
            // 2. decrypt each guess and compare with the commitment
            let mut messages = Vec::new();
            self.participants.iter().for_each(|p| {
                self.messages.get(&p).iter().for_each(|m| {
                    let plaintext = tlock::encryption::encryption::aes_decrypt(m.clone(), &self.nonce, &msk).unwrap();
                    messages.push(plaintext);
                });
            });
            self.revealed_messages = messages;
        }
    }

    // /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    // /// module and test functions are marked with a `#[test]` attribute.
    // /// The below code is technically just normal Rust code.
    // #[cfg(test)]
    // mod tests {
    //     /// Imports all the definitions from the outer scope so we can use them here.
    //     use super::*;

    //     /// We test if the default constructor does its job.
    //     #[ink::test]
    //     fn default_works() {
    //         let sealed_bid_auction = TimelockCommitReveal::default();
    //         assert_eq!(sealed_bid_auction.get(), false);
    //     }

    //     /// We test a simple use case of our contract.
    //     #[ink::test]
    //     fn it_works() {
    //         let mut sealed_bid_auction = SealedBidAuction::new(false);
    //         assert_eq!(sealed_bid_auction.get(), false);
    //         sealed_bid_auction.flip();
    //         assert_eq!(sealed_bid_auction.get(), true);
    //     }
    // }


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
