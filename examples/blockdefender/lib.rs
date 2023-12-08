#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::prelude::vec::Vec;
use etf_chain_extension::ext::EtfEnvironment;

#[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct GameEventClock {
    interval: u8,
    offset: u8,
}

#[ink::contract(env = EtfEnvironment)]
mod vickrey_auction {
    use ink::storage::Mapping;
    use scale::alloc::string::ToString;
    use sha3::Digest;
    use crate::{EtfEnvironment, GameEventClock};

    pub const DEFAULT_ATK: u32 = 100; 
    pub const DEFAULT_DEF: u32 = 100;

    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        AnError,
    }

    /// each player has a 'base'
    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Base {
        /// the amount of iron in the base
        iron: u32,
        /// the number of attack points that the base has
        attack_points: u32,
        /// the number of defense points that the base has
        defense_points: u32,
        /// the physical x coord of the base within the grid
        x_coord: u8,
        /// the physical y coord of the base within the grid
        y_coord: u8,
    }

    impl Base {
        fn new(x: u8, y: u8) -> Self {
            Base {
                iron: 0,
                attack_points: DEFAULT_ATK,
                defense_points: DEFAULT_DEF,
                x_coord: x,
                y_coord: y,
            }
        }
    }

    /// the unique actions that players can take
    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Actions {
        Mine(u8),
        Build(u8),
        Attack(u8),
    }

    /// the auction storage
    #[ink(storage)]
    pub struct BlockDefender {
        /// the max x coordinate of the grid
        x_max: u8,
        /// the max y coordinate of the grid
        y_max: u8,
        /// the maximum number of players that can participate
        /// in any given round
        max_players_per_round: u8,
        /// player attributes
        player_bases: Mapping<AccountId, Base>,
        // / mining event contract
        // mine_event_clock_code_hash: Hash,
        // /// build event contract
        // build_event_clock_code_hash: Hash,
        // /// attack event contract
        // attack_event_clock_code_hash: Hash,
    }

    impl BlockDefender {
    
        /// Constructor that initializes a new game
        #[ink(constructor)]
        pub fn new(x: u8, y: u8, max_players: u8) -> Self {
            Self {
                x_max: x,
                y_max: y,
                max_players_per_round: max_players,
                player_bases: Mapping::default(),
            }
        }

        // // getters for storage
        // #[ink(message)]
        // pub fn get_value(&self) -> u8 {
        //     self.value.clone()
        // }

        // /// start the game schedule feedback loop
        // /// TODO: expose scheduler pallet as chain extension
        // #[ink(message)]
        // pub fn start() {

        // }

        /// create a default base for a new player
        /// we let players choose their own spawn point on the grid
        #[ink(message)]
        pub fn init_player(&mut self, x: u8, y: u8) {
            let caller = self.env().caller();
            if let None = self.player_bases.get(caller) {
                let base = Base::new(x, y);
                self.player_bases.insert(caller, &base);
            }
        }

        #[ink(message)]
        pub fn play(&mut self, action: Actions) {
            
        }

        // #[ink(message)]
        // pub fn 

    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn it_works() {
            let mut contract = BlockDefender::new(1u8, 2u8, 3);
            // assert_eq!(contract.get_value(), 1u8);
        }

        #[ink::test]
        fn can_init_player_with_valid_coordinates() {
            let mut contract = BlockDefender::new(10u8, 10u8, 3);
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            contract.init_player(accounts.alice, 0, 0);
            let expected_player_base = Base::new(0, 0);
            assert_eq!(contract.player_bases.get(accounts.alice).eq(&expected_player_base));
        }

        // #[ink::test]
        // fn init_player_fail_when_duplicate_coordinates() {

        // }

        // #[ink::test]
        // fn init_player_fail_when_coordinates_out_of_bounds() {

        // }


    }
}
