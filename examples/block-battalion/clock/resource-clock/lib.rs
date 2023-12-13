#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::prelude::vec::Vec;
use etf_contract_utils::ext::EtfEnvironment;
pub use self::resource_clock::{
    ResourceClock,
    ResourceClockRef,
};

// TODO: rename to bit roulette?
#[ink::contract(env = EtfEnvironment)]
mod resource_clock {
    use ink::storage::Mapping;
    use scale::alloc::string::ToString;
    use sha3::Digest;
    use etf_contract_utils::types::{
        RoundNumber, 
        SlotNumber, 
        TlockMessage, 
        GameEvent,
        DecryptedData,
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
        NotGameMaster
    }

    /// the auction storage
    #[ink(storage)]
    pub struct ResourceClock {
        /// the controller of the mine clock, calls to the clock must be proxied through it
        game_master: AccountId,
        /// the interval (in slots) that this clock ticks
        interval: SlotNumber,
        /// the initial slot number, when the first event should happen 
        initial_slot: SlotNumber,
        /// the current round number
        current_round: RoundNumber,
        /// a map between rounds (slot ids) and player moves for the upcoming (next) event
        /// this can be cleared after each successive clock advance
        round_input: Mapping<RoundNumber, Vec<(AccountId, TlockMessage)>>,
        /// the amount of IRON each player has
        player_balance: Mapping<AccountId, u32>,
    }

    #[ink(event)]
    pub struct FastForward {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
    }


    impl ResourceClock {
    
        /// Constructor that initializes a new game of roulette
        #[ink(constructor)]
        pub fn new(
            game_master: AccountId,
            config: EventConfig,
        ) -> Self {
            Self {
                game_master,
                interval: config.interval,
                initial_slot: config.initial_slot,
                current_round: 0, 
                round_input: Mapping::default(),
                player_balance: Mapping::default(),
            }
        }

        #[ink(message)]
        pub fn get_current_round_input(&self) -> Option<Vec<(AccountId, TlockMessage)>> {
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
        pub fn get_player_resource_balance(&self, who: AccountId) -> Option<u32> {
            self.player_balance.get(who)
        }

        /// allow the GM to burn some resources
        /// generally used for converting this to a different game resource
        /// conversion is handled by the GM
        #[ink(message)]
        pub fn burn_resource(
            &mut self, 
            player: AccountId, 
            amount: u32,
        ) -> Result<(), Error> {
            verify_game_master(self.env().caller(), self.game_master)?;
            // TODO: ensure only blockbattalion can make this call!
            if let Some(balance) = self.player_balance.get(player) {
                if balance > amount {
                    let new_balance = balance - amount;
                    self.player_balance.insert(player, &new_balance);
                } else {
                    return Err(Error::InvalidResourceAmount)
                }
            }

            Ok(())
        }

        /// place a guess for a future round of roulette
        #[ink(message)]
        pub fn play(
            &mut self,
            player: AccountId,
            input: TlockMessage
        ) -> Result<(), Error> {
            verify_game_master(self.env().caller(), self.game_master)?;
            // TODO: only the block_defender contract should be able to call this contract
            let mut round_input = Vec::new();
            if let Some(mut msgs) = self.round_input.get(self.current_round) {
                    round_input.append(&mut msgs);
            }

            round_input.push((player, input));

            self.round_input.insert(self.current_round, &round_input);
            // TODO: emit event
            Ok(())
        }

        /// advance the clock from the current round to the next one
        #[ink(message)]
        pub fn advance_clock(
            &mut self,
            moves: Vec<DecryptedData<AccountId, u8>>,
        ) -> Result<(), Error> {
            verify_game_master(
                self.env().caller(), 
                self.game_master
            )?;
            // we will ignore future round inputs here
            // that is, this will not support players who want to set a timelocked bit for future events
            // they can only submit messages for 'current' events
            if moves.len() == 0 && self.round_input.get(self.current_round).is_none() {
                let mut to = self.current_round + 1;
                // TODO: could parametrize the num of slots we skip
                (to..to + 3).find(|&t| {
                    let slot = self.initial_slot + t * self.interval;
                    !self.env().extension().check_slot(slot as u64)
                }).map(|t| {
                    self.current_round = t;
                });
                if self.current_round >= to {
                    return Ok(());
                }
            } else {
                // TODO :validations
                // // first we ensure that the input matches the timelock commitment
                // // for now, if any move is invalid we return an error
                // moves.iter().for_each(|m| {
                //     // if there is no commitment for this round, the player did not play
                //     if let Some(message) = self.next_round_input.get(m.0) {
                //         // if the commitment can't be verified, we stop 
                //         let mut b = Vec::new();
                //         b.push(m.1);
                //         if !verify_tlock_commitment(b, m.2, message.commitment) {
                //             // return Err(Error::InvalidCommitment);
                //         }
                //     } else {
                //         // return Err(Error::InvalidPlayer);
                //     }
                // });

                // we won't even check the commitment right now, just directly trust the input
                // self.temp_prev_moves = moves.clone();
                let sum: u8 = moves.iter().map(|decrypted| decrypted.data).sum();
                let parity: u8 = sum % 2;
                let winners: Vec<_> = moves.into_iter()
                    .filter(|decrypted| {
                        // self.temp_counter += 1;
                        decrypted.data == parity
                    })
                    .map(|d| d.address)
                    .collect();
                if winners.len() > 0 {
                    // let iron_per_winner: u32 = 100u32 / (winners.len() as u32);
                    let iron_per_winner = 5;
                    // self.temp_reward = iron_per_winner;
                    // allocate resources
                    winners.iter().for_each(|w| {
                        let mut new_balance = iron_per_winner;
                        if let Some(balance) = self.player_balance.get(w) {
                                new_balance += balance
                        };
                        self.player_balance.insert(w, &new_balance);
                    });
                }

                // cleanup
                // self.round_input.remove(self.current_round);
                self.current_round += 1;
            }
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

    /// verify the timelock commitment 
    pub fn verify_tlock_commitment(
        bytes: Vec<u8>,
        msk: [u8;32],
        commitment: Vec<u8>
    ) -> bool {
        // rebuild the commitment
        let mut hasher = sha3::Sha3_256::new();
        hasher.update(bytes.clone());
        let mut hash = hasher.finalize().to_vec();
        for i in 0..32 {
            hash[i] ^= msk[i];
        }

        // compare against expected hash
        hash.to_vec().eq(&commitment)
    }


    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn can_advance_clock() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut contract = ResourceClock::new(accounts.alice, EventConfig { initial_slot: 1u64, interval: 2u64 });
            // this will need to be updated once I add back the commitment checks
            let mut moves = Vec::new();
            moves.push(DecryptedData {
                address: accounts.alice, // alice wins
                data: 0u8,
                msk: [2;32]
            });
            moves.push(DecryptedData {
                address: accounts.bob,
                data: 1u8,
                msk: [2;32]
            });
            moves.push(DecryptedData {
                address: accounts.charlie, // charlie wins too
                data: 0u8,
                msk: [2;32]
            });
            moves.push(DecryptedData {
                address: accounts.eve,
                data: 1u8,
                msk: [2;32]
            });

            assert_eq!(0, contract.current_round);
            contract.advance_clock(moves).map_err(|e| panic!("Test should not panic"));

            assert_eq!(50, contract.player_balance.get(accounts.alice).unwrap());
            assert_eq!(50, contract.player_balance.get(accounts.charlie).unwrap());
            assert_eq!(1, contract.current_round);

        }
    }

}
