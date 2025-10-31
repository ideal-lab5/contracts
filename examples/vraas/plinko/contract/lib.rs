#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod contract {
    use idn_contracts::prelude::*;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct PlinkoContract {
        idn_client: IdnClient,
        subscription_id: Option<SubscriptionId>,
        latest_randomness: Option<Randomness>,
        pending_games: Vec<(AccountId, Bet)>,
    }

    /// Errors that can occur during Example Consumer contract operations.
	///
	/// This enum covers both contract-specific errors and wrapped errors from the IDN Client
	/// library. All errors implement proper conversion traits to enable seamless error propagation
	/// between the contract and the underlying IDN infrastructure.
    #[allow(clippy::cast_possible_truncation)]
    #[derive(Debug, PartialEq, Eq)]
	#[ink::scale_derive(Encode, Decode, TypeInfo)]
	pub enum ContractError {
        /// An error occurred in the underlying IDN Client during XCM operations or IDN
		/// communication. This wraps errors from cross-chain message sending, execution
		/// failures, or IDN response processing. Check the inner Error for specific
		/// failure details.
		IdnClientError(Error),
		/// Attempted to perform a subscription operation when no active subscription exists.
		/// This occurs when trying to pause, reactivate, update, or terminate a subscription
		/// before creating one, or after a subscription has been terminated.
		NoActiveSubscription,
		/// The caller does not have permission to perform the requested operation.
		/// Only the contract owner can manage subscriptions, and only the configured IDN account
		/// can deliver randomness pulses. This error indicates an authorization check failure.
		Unauthorized,
		/// Attempted to create a subscription when one already exists and is active.
		/// The contract supports only one active subscription at a time. Terminate or update
		/// the existing subscription before creating a new one.
		SubscriptionAlreadyExists,
		/// The provided subscription ID does not match the contract's active subscription.
		/// This occurs when receiving randomness pulses with incorrect subscription identifiers,
		/// indicating potential delivery errors or unauthorized pulse injection attempts.
		InvalidSubscriptionId,
		/// Pulse authentication failed, indicating corrupted or tampered randomness data.
		InvalidPulse,
		/// A general error occurred that doesn't fit into other specific categories.
		/// This includes boundary condition failures, data conversion errors, or unexpected
		/// states that prevent normal contract operation.
		Other,
	}

    type GameResults = Vec<(AccountId, Winnings)>;
    type Winnings = Balance;
    type Randomness = [u8;32];
    type Bet = Balance;
    type Replay = Vec<u8>;


    #[ink(event)]
    pub struct GamePlayed {
        results: GameResults,
        path: Replay,
    }    

    impl PlinkoContract {

        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                idn_client: IdnClient::new(
                    4502,          // IDN parachain ID
                    40,            // IDN Manager pallet index
                    4594,          // Your parachain ID
                    16,            // Contracts pallet index on your chain
                    6,             // Contract callback call index on your chain
                    1_000_000_000, // Maximum XCM execution fees
                ),
                subscription_id: None,
                latest_randomness: None,
                pending_games: Vec::new(),
            }
        }

        #[ink(message)]
		pub fn create_subscription(
			&mut self,
			credits: Credits,
			frequency: IdnBlockNumber,
			metadata: Option<Metadata>,
		) -> Result<SubscriptionId, ContractError> {

			// Only allow creating a subscription if we don't already have one
			if self.subscription_id.is_some() {
				return Err(ContractError::SubscriptionAlreadyExists);
			}

			// Create subscription through IDN client
			let subscription_id = self
				.idn_client
				.create_subscription(credits, frequency, metadata, None, None, None)
				.map_err(ContractError::IdnClientError)?;

			// Update contract state with the new subscription
			self.subscription_id = Some(subscription_id);

			Ok(subscription_id)
		}


        #[ink(message)]
        pub fn get_randomness(
            &mut self, 
        ) -> Result<Option<[u8;32]>, Error> { 
            Ok(self.latest_randomness)
        }

        #[ink(message, payable)]
        pub fn join_game(&mut self, bet: Bet) -> Result<(), Error> {
            let caller = self.env().caller();

            // Add to pending games queue
            self.pending_games.push((caller, bet));
            Ok(())
        }

        #[allow(clippy::arithmetic_side_effects)]
        fn play_plinko(&mut self, randomness: Randomness) {

            let path = Vec::from(byte_to_bit_array(randomness[0]));

            let mut sum: i8 = 0;
            for bit in path.clone() {
                if bit == 0 {
                    sum -= 1;
                } else {
                    sum +=1;
                }
            }

            let multiplier = determine_multiplier(sum);

            let players_and_bets = self.pending_games.clone();
            self.pending_games.clear();

            let mut results = Vec::new();
            for (player, bet) in players_and_bets {
                let winnings = bet * multiplier;
                if winnings > 0 {
                    if self.env().transfer(player, winnings).is_err() {
                        panic!("UHHHHHH THIS IS BAD");
                    } 
                }
                results.push((player, winnings));
            }
            // Emit event with the path for UI playback
            self.env().emit_event(GamePlayed {
                results,
                path,
            });
        }
    }
    

    // Implement the IdnConsumer trait to handle incoming randomness
    impl IdnConsumer for PlinkoContract {
        #[ink(message)]
        fn consume_pulse(
            &mut self, 
            pulse: Pulse,
            subscription_id: SubscriptionId
        ) -> Result<(), Error> {
            let randomness = pulse.rand();
            self.latest_randomness = Some(randomness.clone());
            if self.pending_games.len() > 0 {
                self.play_plinko(randomness);
            }
            Ok(())
        }

        // Handle subscription quotes
        #[ink(message)]
        fn consume_quote(
            &mut self,
            quote: Quote
        ) -> Result<(), Error> {
            Ok(())
        }

        // Handle subscription information responses
        #[ink(message)]
        fn consume_sub_info(
            &mut self,
            sub_info: SubInfoResponse
        ) -> Result<(), Error> {
            Ok(())
        }
    }

    fn byte_to_bit_array(byte: u8) -> [u8; 8] {
        [
            (byte >> 7) & 1,
            (byte >> 6) & 1,
            (byte >> 5) & 1,
            (byte >> 4) & 1,
            (byte >> 3) & 1,
            (byte >> 2) & 1,
            (byte >> 1) & 1,
            byte & 1,
        ]
    }

    fn determine_multiplier(final_sum: i8) -> u128 {
        let abs_sum = final_sum.abs();

        let mut multiplier = 0;

        if abs_sum == 8 {
            multiplier = 5;
        } else if abs_sum == 6 {
            multiplier = 3;
        } else if abs_sum == 4 {
            multiplier = 2
        } else if abs_sum == 2 {
            multiplier = 1
        }

        multiplier
    }
}
