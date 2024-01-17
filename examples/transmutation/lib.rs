#![cfg_attr(not(feature = "std"), no_std, no_main)]

use etf_contract_utils::ext::EtfEnvironment;

#[ink::contract(env = EtfEnvironment)]
mod transmutation {

    use crate::EtfEnvironment;

    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use rs_merkle::{
        algorithms::Sha256,
        Hasher,
        MerkleTree,
    };

    /// a dummy type to represent an asset
    pub type OpaqueAssetId = Vec<u8>;

    /// represents a swap between two participants
    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Swap {
        asset_id_one: OpaqueAssetId,
        asset_id_two: OpaqueAssetId,
        /// the deadline when the swap must complete
        deadline: BlockNumber,
    }

    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        InvalidBlockNumber,
        InvalidMerkleTree,
        SwapDNE,
        DuplicateSeed,
        InvalidSwap,
        NoOwnedAsset,
        NoSuchAsset,
    }

    #[ink(storage)]
    pub struct Transmutation {
        /// mock to track ownership of assets
        /// in real life this like would be 
        /// a reference to some NFT contract
        asset_registry: Mapping<OpaqueAssetId, AccountId>,
        /// a temp registry to hold 'in transit' assets
        asset_status: Mapping<OpaqueAssetId, Hash>,
        /// a collection of all claimed assets
        claimed_assets: Vec<OpaqueAssetId>,
        /// only one asset per account
        // mock_asset_ownership: Mapping<AccountId, OpaqueAssetId>,
        /// a mapping of all swaps
        /// any pair of accounts can only have one active swap
        swaps: Mapping<Hash, Swap>,
        /// a map between account and swaps they can participate in
        pending_swaps: Mapping<AccountId, Hash>,
    }


    impl Transmutation {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor, payable)]
        pub fn new() -> Self {
            Self {
                asset_registry: Mapping::new(),
                asset_status: Mapping::new(),
                claimed_assets: Vec::new(),
                swaps: Mapping::new(),
                pending_swaps: Mapping::new(),
            }
        }

        #[ink(constructor, payable)]
        pub fn default() -> Self {
            Self::new()
        }

        /// generates a random seed
        #[ink(message)]
        pub fn random_seed(
            &mut self,
            input_seed: [u8;48],
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            let mut seed = self.env().extension().secret();
            
            seed.clone().iter().enumerate().for_each(|(i, bit)| {
                seed[i] = *bit ^ input_seed[i];
            });

            if self.claimed_assets.contains(&seed.to_vec()) {
                return Err(Error::DuplicateSeed);
            }

            self.asset_registry.insert(seed.to_vec(), &caller);
            self.claimed_assets.push(seed.to_vec());

            Ok(())
        }

        #[ink(message)]
        pub fn get_asset_swap(&self, asset_id: OpaqueAssetId) -> Option<Hash> {
            self.asset_status.get(asset_id)
        }

        #[ink(message)]
        pub fn get_owner(&self, asset_id: OpaqueAssetId) -> Option<AccountId> {
            self.asset_registry.get(asset_id)
        }

        #[ink(message)]
        pub fn registry_lookup(&self, who: AccountId) -> Option<OpaqueAssetId> {
            if let Some(found_seed) = self.claimed_assets.iter().find(|seed| {
                self.asset_registry
                    .get(seed)
                    .map_or(false, |registry_entry| registry_entry.eq(&who))
            }) {
                return Some(found_seed.clone());
            }
            None
        }
        
        #[ink(message)]
        pub fn get_claimed_assets(&self) -> Vec<OpaqueAssetId> {
            self.claimed_assets.clone()
        }

        #[ink(message)]
        pub fn get_pending_swap(&self) -> Option<Swap> {
            if let Some(hash) = self.pending_swaps.get(self.env().caller()) {
                return self.swaps.get(hash);
            }
            None
        }

        /// get all opens swaps the participant is associated with
        #[ink(message)]
        pub fn swap_lookup(
            &self, 
            left: AccountId, 
            right: AccountId
        ) -> Result<(Hash, Swap), Error> {
            let merkle_root = Self::calculate_merkle_root(left, right)?;
            if let Some(swap) = self.swaps.get(merkle_root)  {
                return Ok((merkle_root, swap));
            }
            Err(Error::SwapDNE)
        }

        /// create a new swap 
        #[ink(message)]
        pub fn try_new_swap( 
            &mut self,
            who: AccountId,
            deadline: BlockNumber,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            // make sure caller has an asset
            if let Some(source_asset_id) = self.registry_lookup(caller.clone()) {
                // and neither asset is part of a pending swap
                if let None = self.pending_swaps.get(caller.clone()) {
                    if let None = self.pending_swaps.get(who.clone()) {
                        // get the owner of the target asset id
                        if let Some(target_asset_id) = self.registry_lookup(who.clone()) {
                            let merkle_root = Self::calculate_merkle_root(caller, who.clone())?;
                            let swap = Swap {
                                asset_id_one: source_asset_id,
                                asset_id_two: target_asset_id,
                                deadline,
                            };
                            let hash = Hash::from(merkle_root);
                            self.swaps.insert(hash, &swap);
                            self.pending_swaps.insert(caller, &hash);
                            self.pending_swaps.insert(who, &hash);
                        } else {
                            return Err(Error::NoSuchAsset);
                        }
                    }
                }
            } else {
                return Err(Error::NoOwnedAsset);
            }
            
            Ok(())
        }

        /// if part of a pending swap, reject it 
        /// this is needed since each participant can have only one pending swap at a time
        #[ink(message)]
        pub fn reject_swap(&mut self) -> Result<(), Error> {
            
            if let Some(_root) = self.pending_swaps.take(self.env().caller()) {
                return Ok(());
            }

            Err(Error::InvalidSwap)
            
        }

        /// transfers ownership of the asset to the contract at the swap deadline only
        #[ink(message)]
        pub fn transmute(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();

            if let Some(merkle_root) = self.pending_swaps.get(caller) {
                if let Some(swap) = self.swaps.get(merkle_root)  {
                    // transmutation must occur simultaneously
                    let current_block = self.env().block_number();
                    if !swap.deadline.eq(&current_block) {
                        return Err(Error::InvalidBlockNumber);
                    }

                    if let Some(asset_owner_one) = 
                        self.asset_registry.get(swap.asset_id_one.clone()) {
                        if asset_owner_one.eq(&caller) {
                            self.asset_status.insert(swap.asset_id_one, &merkle_root);
                        } else {
                            self.asset_status.insert(swap.asset_id_two, &merkle_root);
                        }
                    }
                }
            }
            Ok(())
        }

        #[ink(message)]
        pub fn complete(&mut self, swap_id: Hash) -> Result<(), Error> {
            // let caller = self.env().caller();
            // let merkle_root = Self::calculate_merkle_root(caller, from)?;
            if let Some(swap) = self.swaps.take(swap_id)  {
                let current_block = self.env().block_number();
                if swap.deadline > current_block {
                    return Err(Error::InvalidBlockNumber);
                }
                // both assets  must be locked (r1 and r2 are merkle roots)
                if let Some(r1) = self.asset_status.get(swap.asset_id_one.clone()) {
                    if let Some(r2) = self.asset_status.get(swap.asset_id_two.clone()) {
                        if !r1.eq(&swap_id) || !r2.eq(&swap_id) {
                            return Err(Error::InvalidSwap);
                        }
                    }   
                }
                // execute the swap
                if let Some(asset_owner_one) = self.asset_registry.get(swap.asset_id_one.clone()) {
                    if let Some(asset_owner_two) = self.asset_registry.get(swap.asset_id_two.clone()) {
                        self.asset_registry.insert(swap.asset_id_one.clone(), &asset_owner_two);
                        self.asset_registry.insert(swap.asset_id_two.clone(), &asset_owner_one);
                        self.pending_swaps.remove(asset_owner_one);
                        self.pending_swaps.remove(asset_owner_two);
                        self.asset_status.remove(swap.asset_id_one);
                        self.asset_status.remove(swap.asset_id_two);
                    }
                }
            }

            Ok(())
        }

        /// a helper function to calculate a merkle root
        pub fn calculate_merkle_root(
            left: AccountId, 
            right: AccountId
        ) -> Result<Hash, Error> {
            let mut leaf_values = [left, right];
            let leaves: Vec<[u8;32]> = 
                leaf_values
                    .iter_mut()
                    .map(|x| Sha256::hash(x.as_mut()))
                    .collect();
            let merkle_tree = MerkleTree::<Sha256>::from_leaves(&leaves);
            // this should never happen
            if let Some(merkle_root) = merkle_tree.root() {
                return Ok(Hash::from(merkle_root));
            }
            Err(Error::InvalidMerkleTree)
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
        fn can_register_seed() {
            let accounts = 
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            setup_ext_even_parity();
            let mut transmutation = Transmutation::default();
            assert_eq!(transmutation.swap_lookup(accounts.alice, accounts.bob), Err(Error::SwapDNE));
            assert_eq!(transmutation.claimed_assets.len(), 0);
            if let Err(e) = transmutation.random_seed([5;48]) {
                panic!("{:?}", "The test should pass");
            }

            assert_eq!(transmutation.claimed_assets.len(), 1);
            assert_eq!(
                transmutation.asset_registry.get(transmutation.claimed_assets[0].clone()).unwrap(),
                accounts.alice
            );
        }

        
        #[ink::test]
        fn test_can_create_new_swap() {
            let accounts = 
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            setup_ext_even_parity();
            let mut transmutation = Transmutation::default();

            let deadline = 1;
            
            if let Err(e) = transmutation.random_seed([5;48]) {
                panic!("{:?}", "The test should pass");
            }

            let alice_asset = transmutation.registry_lookup().unwrap();

            // then bob creates one
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            if let Err(e) = transmutation.random_seed([2;48]) {
                panic!("{:?}", "The test should pass");
            }

            let bob_asset = transmutation.registry_lookup().unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            if let Err(e) = transmutation.new_swap(
                alice_asset.clone(), 
                accounts.bob, 
                bob_asset.clone(), 
                deadline
            ) {
                panic!("{:?}", "The test should pass");
            }
            let expected_swap = Swap {
                asset_id_one: alice_asset,
                asset_id_two: bob_asset,
                deadline,
            };

            let merkle_root = Transmutation::calculate_merkle_root(accounts.alice, accounts.bob).unwrap();
            assert_eq!(transmutation.swaps.get(merkle_root).unwrap(), expected_swap);
            assert_eq!(transmutation.swap_lookup(accounts.alice, accounts.bob).unwrap(), (merkle_root, expected_swap));
        }

        #[ink::test]
        fn test_can_trasmute() {
            let accounts = 
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            setup_ext_even_parity();
            let mut transmutation = Transmutation::default();

            let deadline = 1;
            
            if let Err(e) = transmutation.random_seed([5;48]) {
                panic!("{:?}", "The test should pass");
            }

            let alice_asset = transmutation.registry_lookup().unwrap();

            // then bob creates one
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            if let Err(e) = transmutation.random_seed([2;48]) {
                panic!("{:?}", "The test should pass");
            }

            let bob_asset = transmutation.registry_lookup().unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            if let Err(e) = transmutation.new_swap(
                alice_asset.clone(), 
                accounts.bob, 
                bob_asset.clone(), 
                deadline
            ) {
                panic!("{:?}", "The test should pass");
            }
            // let expected_swap = Swap {
            //     asset_id_one: alice_asset,
            //     asset_id_two: bob_asset,
            //     deadline,
            // };

            ink_env::test::advance_block::<ink_env::DefaultEnvironment>();
            if let Err(e) = transmutation.transmute(accounts.bob) {
                panic!("{:?}", "The test should pass");
            }

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            if let Err(e) = transmutation.transmute(accounts.alice) {
                panic!("{:?}", "The test should pass");
            }
            // let merkle_root = Transmutation::calculate_merkle_root(accounts.alice, accounts.bob).unwrap();
            // assert_eq!(transmutation.swaps.get(merkle_root).unwrap(), expected_swap);

        }

        fn setup_ext_even_parity() {
            struct MockETFExtension;
            impl ink_env::test::ChainExtension for MockETFExtension {
                fn func_id(&self) -> u32 {
                    1101
                }

                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    let ret = [0;48];
                    scale::Encode::encode_to(&ret, output);
                    0
                }
            }

            ink_env::test::register_chain_extension(MockETFExtension);
        }
    }


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
    //         let constructor = TransmutationRef::default();

    //         // When
    //         let contract_account_id = client
    //             .instantiate("transmutation", &ink_e2e::alice(), constructor, 0, None)
    //             .await
    //             .expect("instantiate failed")
    //             .account_id;

    //         // Then
    //         let get = build_message::<TransmutationRef>(contract_account_id.clone())
    //             .call(|transmutation| transmutation.get());
    //         let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), false));

    //         Ok(())
    //     }

    //     /// We test that we can read and write a value from the on-chain contract contract.
    //     #[ink_e2e::test]
    //     async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
    //         // Given
    //         let constructor = TransmutationRef::new(false);
    //         let contract_account_id = client
    //             .instantiate("transmutation", &ink_e2e::bob(), constructor, 0, None)
    //             .await
    //             .expect("instantiate failed")
    //             .account_id;

    //         let get = build_message::<TransmutationRef>(contract_account_id.clone())
    //             .call(|transmutation| transmutation.get());
    //         let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), false));

    //         // When
    //         let flip = build_message::<TransmutationRef>(contract_account_id.clone())
    //             .call(|transmutation| transmutation.flip());
    //         let _flip_result = client
    //             .call(&ink_e2e::bob(), flip, 0, None)
    //             .await
    //             .expect("flip failed");

    //         // Then
    //         let get = build_message::<TransmutationRef>(contract_account_id.clone())
    //             .call(|transmutation| transmutation.get());
    //         let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), true));

    //         Ok(())
    //     }
    // }
}
