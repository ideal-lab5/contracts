#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::prelude::{
    // collections::HashSet,
    vec::Vec
};
use etf_contract_utils::ext::EtfEnvironment;

#[ink::contract(env = EtfEnvironment)]
mod block_battalion {
    use ink::{ToAccountId, storage::Mapping};
    // use ink::prelude::collections::HashSet;
    // use ink_prelude::collections::HashSet;
    use hashbrown::HashSet;
    use scale::alloc::string::ToString;
    use sha3::Digest;
    use resource_clock::ResourceClockRef;
    use etf_contract_utils::types::{
        TlockMessage, 
        SlotNumber, 
        DecryptedData,
        EventConfig,
    };
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

    /// represents a player's status
    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Player {
        /// the amount of iron in the base
        iron: u32,
        /// the core of your empire
        core: Base,
    }

    /// each player has a 'base'
    #[derive(PartialEq, Debug, Clone, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Base {
        /// the power level of the base
        power: u32,
        /// the physical x coord of the base "core"
        x: u8,
        /// the physical y coord of the base "core"
        y: u8,
        /// the children of the base, must form a connected graph
        children: Vec<Base>,
    }

    pub const IRON_PER_CELL: u32 = 1u32;
    pub const IRON_PER_PWR_LVL: u32 = 1u32;

    impl Base {
        fn new(x: u8, y: u8) -> Self {
            Base {
                power: 2,
                x: x,
                y: y,
                children: Vec::new(),
            }
        }
    }

    /// a helper struct for dfs
    #[derive(Eq, Hash, PartialEq, Clone)]
    pub struct Point {
        x: u8,
        y: u8,
    }

    /// the unique Events that players can take
    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Events {
        Mine,
        // Enhance(u8),
        // Attack(u8),
    }

    /// the auction storage
    #[ink(storage)]
    pub struct BlockBattalion {
        /// the max x coordinate of the grid 
        x_max: u8,
        /// the max y coordinate of the grid
        y_max: u8,
        /// the maximum number of players that can participate
        /// in any given round
        max_players: u8,
        /// the players
        players: Vec<AccountId>,
        /// player attributes
        player_data: Mapping<AccountId, Player>,
        /// mining event contract
        resource_clock: Option<AccountId>,
        /// really basic grid metadata, stores if the cell is owned or not
        grid_ownership: Mapping<(u8, u8), AccountId>
        // /// build event contract
        // build_event_clock_code_hash: Hash,
        // /// attack event contract
        // attack_event_clock_code_hash: Hash,
    }

    impl BlockBattalion {
    
        /// Constructor that initializes a new game
        #[ink(constructor)]
        pub fn new(
            x: u8, 
            y: u8, 
            max_players: u8, 
        ) -> Self {
            Self {
                x_max: x,
                y_max: y,
                max_players: max_players,
                players: Vec::new(),
                player_data: Mapping::default(),
                resource_clock: None,
                grid_ownership: Mapping::default(),
            }
        }

        #[ink(message)]
        pub fn initialize_event_clock(
            &mut self,
            event: Events,
            code_hash: Hash,
            event_config: EventConfig,
        ) -> Result<(), Error> {
            let contract_addr = self.env().account_id();
            match event {
                Events::Mine => {
                    let resource_clock = ResourceClockRef::new(contract_addr, event_config)
                        .endowment(0)   
                        .code_hash(code_hash)
                        .salt_bytes([0xde, 0xad, 0xbe, 0xef])
                        .instantiate();
                    self.resource_clock = Some(resource_clock.to_account_id());
                }
            }
            Ok(())
        }

        /// get the resource event address if it exists
        #[ink(message)]
        pub fn get_resource_event_address(&self) -> Option<AccountId> {
            self.resource_clock.clone()
        }

        /// get all current players
        #[ink(message)]
        pub fn get_players(&self) -> Vec<AccountId> {
            self.players.clone()
        }

        /// get the player base from the input vec
        #[ink(message)]
        pub fn get_player_base(&self) -> Vec<(AccountId, Base)> {
            self.players
                .iter()
                .filter_map(|player| 
                    self.player_data
                        .get(player)
                        .map(|p| (*player, p.core)))
                .collect()
        }

        /// get the slot when the next event will occur based on the input event
        #[ink(message)]
        pub fn get_next_slot(&self, event: Events) -> SlotNumber {
            match event {
                Events::Mine => {
                    let mut resource_clock_contract: ResourceClockRef =
                        ink::env::call::FromAccountId::from_account_id(
                            self.resource_clock.expect("clock should be initialized").clone());
                    resource_clock_contract.get_next_slot()
                }
            }
        }

        #[ink(message)]
        pub fn get_next_round_input(
            &self, 
            event: Events, 
        ) -> Option<Vec<(AccountId, TlockMessage)>> {
            match event {
                Events::Mine => {
                    let mut resource_clock_contract: ResourceClockRef =
                        ink::env::call::FromAccountId::from_account_id(
                            self.resource_clock.expect("clock should be initialized").clone());
                    resource_clock_contract.get_current_round_input()
                }
            }
        }

        #[ink(message)]
        pub fn get_player_resources(&self, player: AccountId) -> Option<u32> {
            let mut resource_clock_contract: ResourceClockRef =
                ink::env::call::FromAccountId::from_account_id(
                    self.resource_clock.expect("clock should be initialized").clone());
            resource_clock_contract.get_player_resource_balance(player)
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
            if let None = self.player_data.get(caller) {
                let player = Player {
                    iron: 0, 
                    core: Base::new(x, y),
                };
                self.player_data.insert(caller, &player);
                self.players.push(caller);
                self.grid_ownership.insert((x, y), &caller);
            }
        }

        /// attempts to expand a player's base
        /// this initial implementation just uses the 'core' to start, will update 
        #[ink(message)]
        pub fn expand_base(
            &mut self,
            x: u8, 
            y: u8
        ) -> Result<(), Error> {
            let player = self.env().caller();
            let mut resource_clock_contract: ResourceClockRef =
                ink::env::call::FromAccountId::from_account_id(
                    self.resource_clock
                        .expect("event clocks should be initialized")
                        .clone());
            // first check they have sufficient iron
            if let Some (amount) = resource_clock_contract.get_player_resource_balance(player) {
                if amount > IRON_PER_CELL {
                    // then check the cell's ownership
                    // for now we only support conquering unoccupied neighbors.
                    if let None = self.grid_ownership.get((x, y)) {
                        //then we have to verify that it is actually our neighbor
                        if let Some(mut base) = self.player_data.get(player) {
                            if Self::check_graph(base.core.clone(), x, y) {
                                base.core.children.push(Base::new(x, y));
                                // then add a new child to the base
                                self.player_data.insert(player, &base);
                                resource_clock_contract.burn_resource(player, IRON_PER_CELL);
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        #[ink(message)]
        pub fn play(
            &mut self, 
            event: Events, 
            input: TlockMessage,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            match event {
                Events::Mine => {
                    // delegate to mine game clock
                    let mut resource_clock_contract: ResourceClockRef =
                        ink::env::call::FromAccountId::from_account_id(
                            self.resource_clock.expect("clock should be initialized").clone());
                    resource_clock_contract.play(caller, input)
                        .map_err(|err| Error::MineFailed)?;
                }
            }
            Ok(())
        }

        #[ink(message)]
        pub fn advance_clock(
            &mut self, 
            event: Events,
            moves: Vec<DecryptedData<AccountId, u8>>,
        ) -> Result<(), Error> {
            match event {
                Events::Mine => {
                    // delegate to mine game clock
                    let mut resource_clock_contract: ResourceClockRef =
                        ink::env::call::FromAccountId::from_account_id(
                            self.resource_clock.expect("clock should be initialized").clone());               
                    resource_clock_contract.advance_clock(moves)
                        .map_err(|err| {
                            Error::MineAdvanceClockFailed
                        })?;
                }
            }

            Ok(())
        }

        // #[ink(message)]
        // pub fn 


        /// determines if the given point (check_x, check_y) is a neighbor to the 
        /// connected graph formed by the base and its children
        pub fn check_graph(
            core: Base, 
            check_x: u8, 
            check_y: u8,
        ) -> bool {
            // flat map core and child coords
            let mut coords: Vec<Point> = Vec::new();
            let p = Point{ x: core.x, y: core.y };
            coords.push(p);
            coords.append(
                &mut core.children.iter()
                    .map(|child| Point{ x: child.x, y: child.y } )
                    .collect::<Vec<_>>()
                );
            
            Self::is_connected_graph(&coords)
        }

        /// check if the points form a connected graph
        pub fn is_connected_graph(points: &[Point]) -> bool {
            if points.is_empty() {
                return false; // Empty list is not a connected graph
            }
        
            let mut visited = HashSet::new();
            Self::dfs(&points[0], points, &mut visited);
        
            visited.len() == points.len()
        }
        
        pub fn dfs(start: &Point, points: &[Point], visited: &mut HashSet<Point>) {
            if visited.contains(&start.clone()) {
                return;
            }
        
            visited.insert(start.clone());
        
            for point in points {
                if !visited.contains(&point.clone()) && Self::distance(start, &point) == 1 {
                    Self::dfs(&point, points, visited);
                }
            }
        }
        
        pub fn distance(p1: &Point, p2: &Point) -> u8 {
            (p1.x).abs_diff(p2.x) + p1.y.abs_diff(p2.y)
        }
        

    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn it_works() {
            let mut contract = BlockBattalion::new(1u8, 2u8, 3);
            // assert_eq!(contract.get_value(), 1u8);
        }

        #[ink::test]
        fn can_init_player_with_valid_coordinates() {
            let mut contract = BlockBattalion::new(10u8, 10u8, 3);
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            contract.init_player(0, 0);
            let expected_player_data = PlayerData {
                iron: 0,
                core: Base::new(0, 0),
            };
            assert_eq!(contract.player_data.get(accounts.alice).unwrap(), expected_player_data);
        }

        // #[ink::test]
        // fn init_player_fail_when_duplicate_coordinates() {

        // }

        // #[ink::test]
        // fn init_player_fail_when_coordinates_out_of_bounds() {

        // }


    }
}
