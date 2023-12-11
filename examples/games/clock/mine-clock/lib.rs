#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::prelude::vec::Vec;
use etf_contract_utils::ext::EtfEnvironment;
pub use self::mine_clock::{
    MineClock,
    MineClockRef,
};


#[ink::contract(env = EtfEnvironment)]
mod mine_clock {
    use ink::storage::Mapping;
    use scale::alloc::string::ToString;
    use sha3::Digest;
    use etf_contract_utils::types::{
        RoundNumber, 
        SlotNumber, 
        TlockMessage, 
        GameEvent,
        DecryptedData,
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
    }

    /// the auction storage
    #[ink(storage)]
    pub struct MineClock {
        /// the interval (in slots) that this clock ticks
        interval: u8,
        /// the initial slot number, when the first event should happen 
        initial_slot_number: SlotNumber,
        /// the current round number
        current_round: RoundNumber,
        /// a map between rounds (slot ids) and player moves for the upcoming (next) event
        /// this can be cleared after each successive clock advance
        next_round_input: Mapping<AccountId, TlockMessage>,
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


    impl MineClock {
    
        /// Constructor that initializes a new game of roulette
        #[ink(constructor)]
        pub fn new(interval: u8, initial_slot_number: SlotNumber) -> Self {
            Self {
                interval: interval,
                initial_slot_number: initial_slot_number,
                current_round: 0, 
                next_round_input: Mapping::default(),
                player_balance: Mapping::default(),
            }
        }

        #[ink(message)]
        pub fn get_next_round_input(&self, players: Vec<AccountId>) -> Vec<(AccountId, TlockMessage)> {
            players
            .iter()
            .filter_map(|player| 
                self.next_round_input
                    .get(player)
                    .map(|msg| (*player, msg)))
            .collect()
        }

        /// get the next slot number
        #[ink(message)]
        pub fn get_next_slot(&self) -> SlotNumber {
            self.initial_slot_number + (self.current_round * self.interval) as u64
        }

        #[ink(message)]
        pub fn get_current_round(&self) -> RoundNumber {
            self.current_round
        }

        #[ink(message)]
        pub fn get_player_resource_balance(&self, who: AccountId) -> Option<u32> {
            self.player_balance.get(who)
        }

        /// place a guess for a future round of roulette
        #[ink(message)]
        pub fn play(
            &mut self,
            player: AccountId,
            input: TlockMessage
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            // TODO: only the block_defender contract should be able to call this contract
            self.next_round_input.insert(player, &input);
            // allow for guesses to be overwritten if desired
            // overwrites only available before the round has happened 
            // so no block exists in the round slot
            Ok(())
        }

        /// advance the clock from the current round to the next one
        #[ink(message)]
        pub fn advance_clock(
            &mut self,
            moves: Vec<DecryptedData<AccountId, u8>>,
        ) -> Result<(), Error> {
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
            let sum: u8 = moves.iter().map(|decrypted| decrypted.data).sum();
            let parity = sum % 2;
            let winners: Vec<_> = moves.into_iter()
                .filter(|decrypted| decrypted.data == parity)
                .map(|d| d.address)
                .collect();

            let iron_per_winner: u32 = 100u32 / (winners.len() as u32);

            // allocate resources
            winners.iter().for_each(|w| {
                let mut new_balance = iron_per_winner;
                if let Some(balance) = self.player_balance.get(w) {
                        new_balance += balance
                };
                self.player_balance.insert(w, &new_balance);
            });

            // advance to the next round
            self.current_round += 1;
            // clear the next round input
            self.next_round_input = Mapping::default();
            Ok(())
        }

        /// useful when there are consecutive rounds with no input
        /// can skip those rounds and 'fast forward' to the current round
        #[ink(message)]
        pub fn fast_forward(&mut self) -> Result<(), Error> {
            let mut next_slot = self.get_next_slot(); 
            // recursively checks if there is a block and updates the
            // round number until there is not a block
            // for a very large number of round numbers
            // this could result in the contract being trapped 
            // so we probably need a maximum fast forward length
            if self.env().extension().check_slot(next_slot) {
                self.current_round += 1;
                self.fast_forward()?;
            }

            Ok(())
        }
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

}
