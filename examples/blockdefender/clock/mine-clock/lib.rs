#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::prelude::vec::Vec;
use etf_chain_extension::ext::EtfEnvironment;

#[ink::contract(env = EtfEnvironment)]
mod roulette {
    use ink::storage::Mapping;
    use scale::alloc::string::ToString;
    use sha3::Digest;
    use crate::{EtfEnvironment, Vec};


    /// the type to track successive rounds of the game
    /// e.g. {0, 1, 2, 3, ...}
    pub type RoundNumber = u8;

    /// the type to track the slot number associated
    // with game events
    pub type SlotNumber = u64;

    /// a timelocked message
    #[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct TlockMessage {
        /// the ciphertext
        ciphertext: Vec<u8>,
        /// a 12-byte nonce
        nonce: Vec<u8>,
        /// the ibe ciphertext
        capsule: Vec<u8>, // a single ibe ciphertext is expected
        // a timelock commitment
        commitment: Vec<u8>,
    }

    /// represents a new event in the game
    #[derive(Clone, Debug, scale::Decode, scale::Encode, PartialEq)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct GameEvent {
        /// a name to associate with this event
        name: Option<[u8;32]>,
        /// the slot in etf consensus when the event happens
        slot: SlotNumber,
        /// extra data that can be revealed at this slot
        /// as part of an in-game event
        data: Vec<TlockMessage>,
    }

    #[derive(PartialEq, Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum Error {
        /// the caller is not the owner or dealer
        NotOwner,
        /// the specified round has already been completed
        RoundCompleted,
        /// the round secret is not valid
        InvalidRoundSecret,
        /// some player moves are missing, the clock cannot advance
        MissingPlayerMoves,
    }

    /// the auction storage
    #[ink(storage)]
    pub struct Roulette {
        /// the event schedule maps slot numbers to ciphertexts
        /// containing secret, additional data for the round. 
        /// in the roulette, this data is the outcome of the roulette wheel
        /// note that this implies the game is not inherently random or fair
        /// and that is not the intention of this example
        event_schedule: Mapping<RoundNumber, GameEvent>,
        /// a map between rounds (slot ids) and player moves
        /// this can be pruned after each successive clock advance
        guesses: Mapping<(RoundNumber, AccountId), TlockMessage>,
        /// map the round index to the set of winners
        winners: Mapping<u8, Vec<AccountId>>,
        /// track player winnings
        player_balance: Mapping<AccountId, Balance>,
        /// the 'house balance' for making payments to winners
        balance: Balance,
        /// the 'owner' of the casino
        dealer: AccountId,
        /// the current round number
        current_round: u8,
    }

    /// The dealer has set the event schedule
    #[ink(event)]
    pub struct EventScheduleSet { }

    impl Roulette {
    
        /// Constructor that initializes a new game of roulette
        #[ink(constructor)]
        pub fn new(dealer: AccountId) -> Self {
            Self {
                event_schedule: Mapping::default(), 
                guesses: Mapping::default(),
                winners: Mapping::default(),
                player_balance: Mapping::default(),
                balance: 0,
                dealer: dealer,
                current_round: 0,
            }
        }

        /// set the event schedule for the game
        /// only callable by the authorized dealer
        #[ink(message)]
        pub fn set_event_schedule(
            &mut self, 
            events: Vec<(RoundNumber, SlotNumber, GameEvent)>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();
            if !caller.eq(&self.dealer) {
                return Err(Error::NotOwner);
            }
            let mut event_schedule = Mapping::new();
            events.iter().for_each(|g| {
                event_schedule.insert(g.0.clone(), &g.2);
            });
            self.event_schedule = event_schedule;
            self.env().emit_event(EventScheduleSet{});
            Ok(())
        }

        /// add some balance to the casino's wallet
        #[ink(message, payable)]
        pub fn add_balance(&mut self) {
            let value = self.env().transferred_value();
            self.balance = value;
        }

/// place a guess for a future round of roulette
#[ink(message)]
pub fn guess(
    &mut self, 
    round: RoundNumber, 
    guess: TlockMessage
) -> Result<(), Error> {
    let caller = self.env().caller();
    // allow for guesses to be overwritten if desired
    // overwrites only available before the round has happened 
    // so no block exists in the round slot
    if let Some(event) = self.event_schedule.get(round) {
        if self.env().extension().check_slot(event.slot) {
            return Err(Error::RoundCompleted)
        }
        self.guesses.insert((round, caller), &guess);
    }
    
    Ok(())
}

        /// advance the clock from the current round to the next one
        #[ink(message)]
        pub fn advance_clock(
            &mut self,
            round_secret: (u8, [u8;32]),
            moves: Vec<(AccountId, u8, [u8;32])>,
        ) -> Result<(), Error> {
            if let Some(game_event) = self.event_schedule.get(self.current_round) {
                // ensure clock advancement is legal
                if self.env().extension().check_slot(game_event.slot) {
                    return Err(Error::RoundCompleted)
                }
            
                let mut input = Vec::new();
                input.push(round_secret.0);
                if !verify_tlock_commitment(
                    input, 
                    round_secret.1, 
                    game_event.data[0].commitment.clone()
                ) {
                    return Err(Error::InvalidRoundSecret)
                }

                // a vec to track any input moves for players that didn't play in the round
                let mut bad_moves: Vec<(AccountId, u8, [u8;32])> = Vec::new();
                // a vec to track any moves where the calculated hash does not match the expected one
                let mut error_moves: Vec<(AccountId, u8, [u8;32])> = Vec::new();

                let mut winners: Vec<AccountId> = Vec::new();

                // for now, we assume that all moves must be provided at once
                let mut number_valid_moves = 0;

                moves.iter().for_each(|m| {
                    // fetch all the plays comitted to for the round
                    if let Some(guess) = self.guesses.get((self.current_round, m.0)) {
                        let c = guess.commitment;
                        let mut input = Vec::new();
                        input.push(m.1);
                        if !verify_tlock_commitment(
                            input,
                            m.2,
                            c,
                        ) {
                            error_moves.push(*m);
                        } else {
                            number_valid_moves += 1;
                            if m.1.eq(&round_secret.0) {
                                let mut current_winners = self.winners.get(self.current_round).expect("should exist");
                                current_winners.push(m.0);
                                self.winners.insert(self.current_round, &current_winners);
                                self.balance -= 1;
                                let mut new_balance = 1;
                                if let Some (balance) = self.player_balance.get(m.0) {
                                    new_balance += balance;
                                } else {
                                    self.player_balance.insert(m.0, &new_balance);
                                }
                            }
                        }
                    };
                });

                if number_valid_moves != moves.len() {
                    return Err(Error::MissingPlayerMoves)
                }
                self.current_round += 1;
                
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
