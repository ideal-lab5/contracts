#![cfg_attr(not(feature = "std"), no_std, no_main)]

use idn_contracts::ext::IDNEnvironment;

#[ink::contract(env = IDNEnvironment)]
mod gacha_nft {
    use super::*;
    use idn_contracts::{
        ext::RandomReadErr,
        vraas::select,
    };
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    pub type NFTId = u32;
    pub type TraitId = u8;

    #[ink(storage)]
    pub struct GachaNFT {
        /// NFT ID -> owner mapping
        nft_owners: Mapping<NFTId, AccountId>,
        /// NFT ID -> list of trait IDs
        nft_traits: Mapping<NFTId, Vec<TraitId>>,
        /// Next NFT ID to mint
        next_nft_id: NFTId,
        /// Mint price
        mint_price: Balance,
        /// Spin price
        spin_price: Balance,
    }

    #[ink(event)]
    pub struct NFTMinted {
        #[ink(topic)]
        nft_id: NFTId,
        #[ink(topic)]
        owner: AccountId,
        traits: Vec<TraitId>,
    }

    #[ink(event)]
    pub struct TraitAdded {
        #[ink(topic)]
        nft_id: NFTId,
        #[ink(topic)]
        owner: AccountId,
        new_trait: TraitId,
    }

    #[derive(Debug, PartialEq, Eq, codec::Encode, codec::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Not the owner of the NFT
        NotOwner,
        /// NFT does not exist
        NFTNotFound,
        /// Insufficient payment
        InsufficientPayment,
        /// Random generation failed
        RandomFailed,
        /// Transfer failed
        TransferFailed,
    }

    impl GachaNFT {
        #[ink(constructor)]
        pub fn new(mint_price: Balance, spin_price: Balance) -> Self {
            Self {
                nft_owners: Mapping::default(),
                nft_traits: Mapping::default(),
                next_nft_id: 1,
                mint_price,
                spin_price,
            }
        }

        #[ink(constructor)]
        pub fn new_default() -> Self {
            Self::new(
                1_000_000_000_000, // 1 token for mint
                100_000_000_000,   // 0.1 token for spin
            )
        }

        /// Mint a new NFT with random starting traits
        #[ink(message, payable)]
        pub fn mint_nft(&mut self) -> Result<NFTId, Error> {
            let caller = self.env().caller();
            let payment = self.env().transferred_value();

            // Check payment
            if payment < self.mint_price {
                return Err(Error::InsufficientPayment);
            }

            let nft_id = self.next_nft_id;
            self.next_nft_id.saturating_add(1);

            // Generate 2-3 random starting traits
            let starting_traits = self.generate_starting_traits(caller)?;

            // Store the NFT
            self.nft_owners.insert(nft_id, &caller);
            self.nft_traits.insert(nft_id, &starting_traits);

            // Emit event
            self.env().emit_event(NFTMinted {
                nft_id,
                owner: caller,
                traits: starting_traits,
            });

            Ok(nft_id)
        }

        /// Spin the gacha to get a new random trait for your NFT
        #[ink(message, payable)]
        pub fn spin_gacha(&mut self, nft_id: NFTId) -> Result<TraitId, Error> {
            let caller = self.env().caller();
            let payment = self.env().transferred_value();

            // Check payment
            if payment < self.spin_price {
                return Err(Error::InsufficientPayment);
            }

            // Check ownership
            let owner = self.nft_owners.get(nft_id).ok_or(Error::NFTNotFound)?;
            if owner != caller {
                return Err(Error::NotOwner);
            }

            // Generate new random trait
            let new_trait = self.generate_random_trait(caller)?;

            // Add trait to NFT (allow duplicates for simplicity)
            let mut traits = self.nft_traits.get(nft_id).unwrap_or_default();
            traits.push(new_trait);
            self.nft_traits.insert(nft_id, &traits);

            // Emit event
            self.env().emit_event(TraitAdded {
                nft_id,
                owner: caller,
                new_trait,
            });

            Ok(new_trait)
        }

        /// Get the traits of an NFT
        #[ink(message)]
        pub fn get_nft_traits(&self, nft_id: NFTId) -> Option<Vec<TraitId>> {
            self.nft_traits.get(nft_id)
        }

        /// Get the owner of an NFT
        #[ink(message)]
        pub fn get_nft_owner(&self, nft_id: NFTId) -> Option<AccountId> {
            self.nft_owners.get(nft_id)
        }

        /// Get NFTs owned by an account
        #[ink(message)]
        pub fn get_owned_nfts(&self, owner: AccountId) -> Vec<NFTId> {
            let mut owned = Vec::new();
            for id in 1..self.next_nft_id {
                if let Some(nft_owner) = self.nft_owners.get(id) {
                    if nft_owner == owner {
                        owned.push(id);
                    }
                }
            }
            owned
        }

        /// Get current prices
        #[ink(message)]
        pub fn get_prices(&self) -> (Balance, Balance) {
            (self.mint_price, self.spin_price)
        }

        /// Generate random starting traits (2-3 traits)
        fn generate_starting_traits(&self, caller: AccountId) -> Result<Vec<TraitId>, Error> {
            let acct_id_bytes: &[u8] = caller.as_ref();
            
            // Create data pool for selection (trait IDs 1-50 for variety)
            let trait_pool: Vec<u8> = (1..=50).collect();
            
            // Select 3 random traits
            let selected = select(
                self.env(),
                trait_pool,
                acct_id_bytes.try_into().unwrap(),
                3,
            ).map_err(|_| Error::RandomFailed)?;

            Ok(selected)
        }

        /// Generate a single random trait
        fn generate_random_trait(&self, caller: AccountId) -> Result<TraitId, Error> {
            let acct_id_bytes: &[u8] = caller.as_ref();
            
            // Create weighted trait pool (lower numbers = more common)
            let mut trait_pool = Vec::new();
            
            // Common traits (1-20): 70% chance
            for _ in 0..7 {
                trait_pool.extend_from_slice(&(1..=20).collect::<Vec<u8>>());
            }
            
            // Rare traits (21-40): 25% chance  
            for _ in 0..25 {
                trait_pool.extend_from_slice(&(21..=40).collect::<Vec<u8>>());
            }
            
            // Epic traits (41-50): 5% chance
            for _ in 0..5 {
                trait_pool.extend_from_slice(&(41..=50).collect::<Vec<u8>>());
            }

            // Select 1 random trait
            let selected = select(
                self.env(),
                trait_pool,
                acct_id_bytes.try_into().unwrap(),
                1,
            ).map_err(|_| Error::RandomFailed)?;

            Ok(selected[0])
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;
        use codec::Encode;

        fn setup_mock_extension() {
            pub struct MockRandExtension;

            impl test::ChainExtension for MockRandExtension {
                fn ext_id(&self) -> u16 {
                    42
                }

                fn call(&mut self, _func_id: u16, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    // Mock random seed
                    let seed: [u8; 32] = [0x42; 32];
                    output.extend(seed.encode());
                    0
                }
            }

            test::register_chain_extension(MockRandExtension);
        }

        #[ink::test]
        fn test_mint_nft_works() {
            setup_mock_extension();
            let mut contract = GachaNFT::new_default();
            
            // Set up account with sufficient balance
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000_000_000);

            let result = contract.mint_nft();
            assert!(result.is_ok());
            
            let nft_id = result.unwrap();
            assert_eq!(nft_id, 1);
            
            // Check ownership
            assert_eq!(contract.get_nft_owner(nft_id), Some(accounts.alice));
            
            // Check traits were generated
            let traits = contract.get_nft_traits(nft_id);
            assert!(traits.is_some());
            assert_eq!(traits.unwrap().len(), 3);
        }

        #[ink::test]
        fn test_spin_gacha_works() {
            setup_mock_extension();
            let mut contract = GachaNFT::new_default();
            
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            
            // First mint an NFT
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000_000_000);
            let nft_id = contract.mint_nft().unwrap();
            
            let initial_traits = contract.get_nft_traits(nft_id).unwrap();
            let initial_count = initial_traits.len();
            
            // Now spin gacha
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100_000_000_000);
            let result = contract.spin_gacha(nft_id);
            assert!(result.is_ok());
            
            // Check that a new trait was added
            let new_traits = contract.get_nft_traits(nft_id).unwrap();
            assert_eq!(new_traits.len(), initial_count + 1);
        }

        #[ink::test]
        fn test_insufficient_payment_fails() {
            setup_mock_extension();
            let mut contract = GachaNFT::new_default();
            
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(500_000_000_000); // Too low
            
            let result = contract.mint_nft();
            assert_eq!(result, Err(Error::InsufficientPayment));
        }

        #[ink::test]
        fn test_non_owner_cannot_spin() {
            setup_mock_extension();
            let mut contract = GachaNFT::new_default();
            
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            
            // Alice mints NFT
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000_000_000);
            let nft_id = contract.mint_nft().unwrap();
            
            // Bob tries to spin Alice's NFT
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(100_000_000_000);
            let result = contract.spin_gacha(nft_id);
            assert_eq!(result, Err(Error::NotOwner));
        }
    }
}