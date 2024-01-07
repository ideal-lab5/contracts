#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::prelude::vec::Vec;
use etf_contract_utils::ext::EtfEnvironment;
pub use self::bit_roulette::{
    BitRoulette,
    BitRouletteRef,
};

#[ink::contract(env = EtfEnvironment)]
mod bit_roulette {
    use ink::storage::Mapping;
    // use sha3::Digest;
    use etf_contract_utils::types::{
        RoundNumber, 
        SlotNumber,
        EventConfig,
    };
    use crate::{EtfEnvironment, Vec};

    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        InvalidCommitment,
        InvalidPlayer,
        Test(bool),
        InvalidRoundNumber,
        InputExists(RoundNumber),
        InvalidResourceAmount,
        NotGameMaster,
        InvalidBlockNumber
    }

    /// the auction storage
    #[ink(storage)]
    pub struct BitRoulette {
        /// the controller of the mine clock, calls to the clock must be proxied through it
        game_master: AccountId,
        /// the block number when the contract was created
        created_at: BlockNumber,
        /// the interval (in slots) that this clock ticks
        interval: SlotNumber,
        /// the initial slot number, when the first event should happen 
        initial_slot: SlotNumber,
        /// the current round number
        current_round: RoundNumber,
        /// a map between rounds (slot ids) and player moves for the upcoming (next) event
        /// this can be cleared after each successive clock advance
        round_input: Mapping<RoundNumber, Vec<(AccountId, u8)>>,
        /// the amount of IRON each player has
        results: Mapping<AccountId, Vec<(RoundNumber, u8)>>,
    }

    impl BitRoulette {
    
        /// TODO: interval must be non-zero
        /// Constructor that initializes a new game of roulette
        #[ink(constructor)]
        pub fn new(
            game_master: AccountId,
            config: EventConfig,
            start_at: BlockNumber,
        ) -> Self {
            Self {
                game_master,
                created_at: start_at,
                interval: config.interval,
                initial_slot: config.initial_slot,
                current_round: 0, 
                round_input: Mapping::default(),
                results: Mapping::default(),
            }
        }

        #[ink(message)]
        pub fn get_current_round_input(&self) -> Option<Vec<(AccountId, u8)>> {
            self.round_input.get(&self.current_round)
        }

        /// get the next slot number
        #[ink(message)]
        pub fn get_next_slot(&self) -> SlotNumber {
            self.initial_slot + self.current_round as u64 * self.interval
        }

        #[ink(message)]
        pub fn get_current_round(&self) -> RoundNumber {
            self.current_round
        }

        #[ink(message)]
        pub fn get_results(
            &self, 
            who: AccountId, 
            round: Option<RoundNumber>,
        ) -> Option<Vec<(RoundNumber, u8)>> {
            let optional_results: Option<Vec<(RoundNumber, u8)>> = self.results.get(who);

            if round.is_none() {
                return optional_results;
            }

            if let Some(results) = optional_results {
                if let Some(r) = round {
                    return Some(results.iter()
                        .filter(|res| res.0.eq(&r))
                        .map(|res| *res)
                        .collect::<Vec<_>>()
                    );
                }
            }
            None
        }

        /// place a guess for a future round of roulette
        #[ink(message)]
        pub fn play(
            &mut self,
            player: AccountId,
            input: u8
        ) -> Result<(), Error> {
            verify_game_master(self.env().caller(), self.game_master)?;
            // we need to make sure it's the right time to call this function
            let current_block = self.env().block_number();
            // fast forward to the closest valid round number
            let diff = current_block.saturating_sub(self.created_at);
            let actual_slot_number = self.initial_slot + diff as u64;
            while self.current_round * self.interval < diff as u64 {
                self.current_round += 1;
            }
            let expected_next_slot_number = 
                self.initial_slot + self.interval * self.current_round;
            
            if !expected_next_slot_number.eq(&actual_slot_number) {
                return Err(Error::InvalidBlockNumber);
            }
            // calculates the parity from the expected next slot number
            // TODO: should check that it is not all 0's (invalid slot)
            let mut parity: u8 = self.env()
                .extension()
                .secret(expected_next_slot_number)
                .to_vec()
                .iter()
                .sum();
            parity = parity % 2;
            let mut player_results = Vec::new();
            
            if let Some(mut player_data) = self.results.get(player) {
                player_results.append(&mut player_data);
            }
            if parity.eq(&(input % 2)) {
                player_results.push((self.current_round, 1));
            } else {
                player_results.push((self.current_round, 0));
            }

            self.results.insert(player, &player_results);

            Ok(())
        }
    }

    /// check if the account is the clock's game master
    pub fn verify_game_master(
        who: AccountId, 
        game_master: AccountId
    ) -> Result<(), Error> {
        if !who.eq(&game_master) {
            return Err(Error::NotGameMaster);
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn clock_can_play_with_single_player() {
            let accounts = 
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut contract = 
                BitRoulette::new(
                    accounts.alice, 
                    EventConfig { 
                        initial_slot: 0u64, 
                        interval: 1u64,
                    },
                    0,
                );
    
            setup_ext_even_parity();
            assert_eq!(None, contract.results.get(accounts.alice));
            ink_env::test::advance_block::<ink_env::DefaultEnvironment>();
            let _ = contract.play(accounts.alice, 0)
                .map_err(|_| panic!("{:?}", "the call should work"));
            
            let mut expected_result = Vec::new();
            expected_result.push((1u64, 1u8));
            assert_eq!(expected_result, contract.results
                            .get(accounts.alice)
                            .unwrap());
        }

        #[ink::test]
        fn clock_can_play_with_many_players() {
            let accounts = 
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut contract = 
                BitRoulette::new(
                    accounts.alice, 
                    EventConfig { 
                        initial_slot: 0u64, 
                        interval: 1u64,
                    },
                    0,
                );

            setup_ext_odd_parity();
            ink_env::test::advance_block::<ink_env::DefaultEnvironment>();
            // odd parity => only bob wins 
            let _ = contract.play(accounts.alice, 0).map_err(|_| panic!("{:?}", "the call should work"));
            let _ = contract.play(accounts.bob, 1).map_err(|_| panic!("{:?}", "the call should work"));
            let _ = contract.play(accounts.charlie, 0).map_err(|_| panic!("{:?}", "the call should work"));

            let mut expected_fail = Vec::new();
            expected_fail.push((1u64, 0u8));

            let mut expected_result = Vec::new();
            expected_result.push((1u64, 1u8));

            assert_eq!(expected_fail, contract.results
                            .get(accounts.alice)
                            .unwrap());

            assert_eq!(expected_result, contract.results
                .get(accounts.bob)
                .unwrap());

            assert_eq!(expected_fail, contract.results
                .get(accounts.charlie)
                .unwrap());
        }


        #[ink::test]
        fn clock_fails_when_executed_at_invalid_block() {
            let accounts = 
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut contract = 
                BitRoulette::new(
                    accounts.alice, 
                    EventConfig { 
                        initial_slot: 1u64, 
                        interval: 2u64
                    },
                    1,
                );
            setup_ext_even_parity();
            // the slot/block schedule is 1, 3, 5, 7, ... and so on. all odd numbers
            // jump ahead to block number 2
            ink_env::test::advance_block::<ink_env::DefaultEnvironment>();
            ink_env::test::advance_block::<ink_env::DefaultEnvironment>();
            match contract.play(accounts.alice, 0) {
                Ok(_) => {
                    panic!("{:?}", "we should have encountered an error");
                },
                Err(e) => {
                    assert_eq!(e, Error::InvalidBlockNumber);
                }
            }
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

        fn setup_ext_odd_parity() {
            struct MockETFExtension;
            impl ink_env::test::ChainExtension for MockETFExtension {
                fn func_id(&self) -> u32 {
                    1101
                }

                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    let mut ret = [1;48];
                    ret[0] = 0;
                    scale::Encode::encode_to(&ret, output);
                    0
                }
            }

            ink_env::test::register_chain_extension(MockETFExtension);
        }
    }

}
