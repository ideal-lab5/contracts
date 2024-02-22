#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::prelude::vec::Vec;
use etf_contract_utils::ext::EtfEnvironment;

#[ink::contract(env = EtfEnvironment)]
mod world_regsistry {
    use ink::storage::Mapping;
    use crate::{EtfEnvironment, Vec};
    /// an identifier for worlds
    pub type WorldId = [u8;48];

    /// an onchain world
    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct World {
        owner: AccountId,
        name: Vec<u8>,
    }

    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        /// the origin must match the configured proxy
        DuplicateWorldId,
    }

    /// the auction storage
    #[ink(storage)]
    pub struct WorldRegistry {
        /// a mapping of owned world ids
        ownership: Mapping<AccountId, Vec<WorldId>>,
        /// a mapping of all worlds
        worlds: Mapping<WorldId, World>,
    }

    impl WorldRegistry {
    
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                ownership: Mapping::default(),
                worlds: Mapping::default(),
            }
        }

        #[ink(message)]
        pub fn get_world(&self, world_id: WorldId) -> Option<World> {
            self.worlds.get(world_id)
        }

        /// create a random seed 
        /// "create a server"
        #[ink(message)]
        pub fn random_seed(
            &mut self,
            name: Vec<u8>, 
            input_seed: [u8;48],
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            // get the latest slot secret as a source of randomness
            let mut seed: WorldId = self.env()
                .extension()
                .secret();
            // we want to try to generate unique noise
            seed.clone().iter().enumerate().for_each(|(i, bit)| {
                seed[i] = *bit ^ input_seed[i];
            });
            
            // this is EXTREMELY unlikely to happen
            if let Some(_world) = self.worlds.get(seed) {
                return Err(Error::DuplicateWorldId);
            }
            self.worlds.insert(seed, &World { owner: caller, name });
            
            let mut owned = Vec::new();
            if let Some(mut o) = self.ownership.get(caller) {
                owned.append(&mut o);
            }
            owned.push(seed);
            self.ownership.insert(caller, &owned);
            Ok(())
        }

    }

    #[cfg(test)]
    mod tests {
        // use super::*;

        // #[ink::test]
        // fn it_works() {
        //     let mut contract = Template::new(1u8);
        //     assert_eq!(contract.get_value(), 1u8);
        // }
    }
}
