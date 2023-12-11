#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::prelude::vec::Vec;
use etf_contract_utils::ext::EtfEnvironment;

#[ink::contract(env = EtfEnvironment)]
mod block_defender {
    use ink::{ToAccountId, storage::Mapping};
    use scale::alloc::string::ToString;
    use sha3::Digest;
    use mine_clock::MineClockRef;
    use etf_contract_utils::types::{TlockMessage, SlotNumber, DecryptedData};
    use crate::{EtfEnvironment, Vec};

    pub const DEFAULT_ATK: u32 = 100; 
    pub const DEFAULT_DEF: u32 = 100;

    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        MineFailed,
        MineAdvanceClockFailed,
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
        atk: u32,
        /// the number of defense points that the base has
        def: u32,
        /// the physical x coord of the base "core"
        x: u8,
        /// the physical y coord of the base "core"
        y: u8,
        /// the children of the base, must form a connected graph
        children: Vec<(u8, u8)>,
    }

    impl Base {
        fn new(x: u8, y: u8) -> Self {
            Base {
                iron: 0,
                atk: DEFAULT_ATK,
                def: DEFAULT_DEF,
                x: x,
                y: y,
                children: Vec::new(),
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
        Mine,
        // Enhance(u8),
        // Attack(u8),
    }

    /// the auction storage
    #[ink(storage)]
    pub struct BlockDefender {
        /// the max x coordinate of the gridhttps://discord.com/channels/@me/855916879969517578
        x_max: u8,
        /// the max y coordinate of the grid
        y_max: u8,
        /// the maximum number of players that can participate
        /// in any given round
        max_players: u8,
        /// the players
        players: Vec<AccountId>,
        /// player attributes
        player_bases: Mapping<AccountId, Base>,
        // / mining event contract
        mine_clock: AccountId,
        // /// build event contract
        // build_event_clock_code_hash: Hash,
        // /// attack event contract
        // attack_event_clock_code_hash: Hash,
    }

    impl BlockDefender {
    
        /// Constructor that initializes a new game
        #[ink(constructor)]
        pub fn new(
            x: u8, y: u8, 
            max_players: u8, 
            mine_clock_code_hash: Hash,
            mine_start_slot: SlotNumber,
        ) -> Self {
            let mine_clock = MineClockRef::new(10, mine_start_slot)
                .endowment(0)   
                .code_hash(mine_clock_code_hash)
                .salt_bytes([0xde, 0xad, 0xbe, 0xef])
                .instantiate();
            
            Self {
                x_max: x,
                y_max: y,
                max_players: max_players,
                players: Vec::new(),
                player_bases: Mapping::default(),
                mine_clock: mine_clock.to_account_id(),
            }
        }

        #[ink(message)]
        pub fn get_players(&self) -> Vec<AccountId> {
            self.players.clone()
        }

        /// get the player bases from the input vec
        #[ink(message)]
        pub fn get_player_base(&self) -> Vec<(AccountId, Base)> {
            self.players
                .iter()
                .filter_map(|player| 
                    self.player_bases
                        .get(player)
                        .map(|base| (*player, base)))
                .collect()
        }

        /// get the slot when the next event will occur based on the input action
        #[ink(message)]
        pub fn get_next_slot(&self, action: Actions) -> SlotNumber {
            match action {
                Actions::Mine => {
                    let mut mine_clock_contract: MineClockRef =
                        ink::env::call::FromAccountId::from_account_id(self.mine_clock.clone());
                    mine_clock_contract.get_next_slot()
                }
            }
        }

        #[ink(message)]
        pub fn get_next_round_input(
            &self, 
            action: Actions, 
        ) -> Vec<(AccountId, TlockMessage)> {
            match action {
                Actions::Mine => {
                    let mut mine_clock_contract: MineClockRef =
                        ink::env::call::FromAccountId::from_account_id(self.mine_clock.clone());
                    mine_clock_contract.get_next_round_input(self.players.clone())
                }
            }
        }

        // // /// start the game schedule feedback loop
        // // /// TODO: expose scheduler pallet as chain extension?
        // #[ink(message)]
        // pub fn start(&mut self, ) {
        //     // first we init the clocks
          
        // }

        /// create a default base for a new player
        /// we let players choose their own spawn point on the grid
        #[ink(message)]
        pub fn init_player(&mut self, x: u8, y: u8) {
            let caller = self.env().caller();
            if let None = self.player_bases.get(caller) {
                let base = Base::new(x, y);
                self.player_bases.insert(caller, &base);
                self.players.push(caller);
            }
        }

        #[ink(message)]
        pub fn play(&mut self, action: Actions, input: TlockMessage) -> Result<(), Error> {
            let caller = self.env().caller();
            match action {
                Actions::Mine => {
                    // delegate to mine game clock
                    let mut mine_clock_contract: MineClockRef =
                        ink::env::call::FromAccountId::from_account_id(self.mine_clock.clone());
                    mine_clock_contract.play(caller, input)
                        .map_err(|err| Error::MineFailed)?;
                }
            }
            Ok(())
        }

        #[ink(message)]
        pub fn advance_clock(
            &mut self, 
            action: Actions,
            moves: Vec<DecryptedData<AccountId, u8>> // I need a better name for this...
        ) -> Result<(), Error> {
            match action {
                Actions::Mine => {
                    // delegate to mine game clock
                    let mut mine_clock_contract: MineClockRef =
                        ink::env::call::FromAccountId::from_account_id(self.mine_clock.clone());               
                    // if empty vec passed, attempt to fast forward the event clock
                    // this is horribly dangerous, but w/e i'm just
                    // seeting if it could work from my UI
                    if moves.len() == 0 {
                        mine_clock_contract.fast_forward()
                        .map_err(|err| {
                            self.players = Vec::new();
                        });
                    } else {
                        mine_clock_contract.advance_clock(moves)
                            .map_err(|err| {
                                // just goofin'
                                self.players = Vec::new();
                            });
                    }                        
                }
            }

            Ok(())
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
            contract.init_player(0, 0);
            let expected_player_base = Base::new(0, 0);
            assert_eq!(contract.player_bases.get(accounts.alice).unwrap(), expected_player_base);
        }

        // #[ink::test]
        // fn init_player_fail_when_duplicate_coordinates() {

        // }

        // #[ink::test]
        // fn init_player_fail_when_coordinates_out_of_bounds() {

        // }


    }
}
