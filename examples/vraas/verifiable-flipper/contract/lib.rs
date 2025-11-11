#![cfg_attr(not(feature = "std"), no_std, no_main)]
use idn_contracts::prelude::*;

#[allow(clippy::arithmetic_side_effects)]
#[ink::contract]
mod coin_flip {
   
    use super::*;
    use ink::storage::Mapping;
    use scale_info::prelude::vec::Vec;

    type Rand = <Pulse as TPulse>::Rand;
   
    #[ink(storage)]
    pub struct CoinFlip {
        idn_client: IdnClient,
        subscription_id: Option<SubscriptionId>,
        latest_rand: Option<Rand>,
        // the current round of the verifiable flipper 
        round: u64,
        // Map round number to winners
        winners: Mapping<u64, Vec<AccountId>>,
        // (player, bet, heads_or_tails)
        pending_flips: Vec<(AccountId, Balance, bool)>,
    }
   
    #[ink(event)]
    pub struct FlipResult {
        #[ink(topic)]
        player: AccountId,
        #[ink(topic)]
        round: u64,
        won: bool,
        result: bool,
    }
   
    impl CoinFlip {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                idn_client: IdnClient::new(4502, 40, 4594, 16, 6, 10000000),
                subscription_id: None,
                latest_rand: None,
                round: 0u64,
                winners: Mapping::new(),
                pending_flips: Vec::new(),
            }
        }
       
        #[ink(message)]
        pub fn create_subscription(&mut self) {
            self.subscription_id = Some(
                self.idn_client.create_subscription(100, 10, None, None, None, None).unwrap()
            );
        }
       
        #[ink(message, payable)]
        pub fn flip(&mut self, call_heads: bool) {
            let bet = self.env().transferred_value();
            self.pending_flips.push((self.env().caller(), bet, call_heads));
        }
        
        // Getters
        #[ink(message)]
        pub fn get_winners(&self, round: u64) -> Option<Vec<AccountId>> {
            self.winners.get(round)
        }
        
        #[ink(message)]
        pub fn get_pending_flips(&self) -> Vec<(AccountId, Balance, bool)> {
            self.pending_flips.clone()
        }
        
        #[ink(message)]
        pub fn get_subscription_id(&self) -> Option<SubscriptionId> {
            self.subscription_id
        }

                        
        #[ink(message)]
        pub fn get_latest_rand(&self) -> Option<Rand> {
            self.latest_rand
        }
                
        #[ink(message)]
        pub fn get_round(&self) -> u64 {
            self.round
        }
    }
   
    impl IdnConsumer for CoinFlip {
        #[ink(message)]
        fn consume_pulse(&mut self, pulse: Pulse, _subscription_id: SubscriptionId) -> IDNResult {
            let randomness = pulse.rand();
            self.latest_rand = Some(randomness);
            let round = self.round;

            let pending = self.pending_flips.clone();
            self.pending_flips.clear();
            
            let mut round_winners = Vec::new();
           
            for (idx, (player, _bet, call)) in pending.iter().enumerate() {
                let result = randomness[idx % randomness.len()] % 2 == 0;
                let won = result == *call;
               
                if won {
                    round_winners.push(*player);
                    // uncomment this to reward based on the bet
                    // self.env().transfer(*player, bet.saturating_mul(2)).ok();
                }
               
                self.env().emit_event(FlipResult { 
                    player: *player, 
                    round,
                    won, 
                    result 
                });
            }
            
            // Store winners for this round
            if !round_winners.is_empty() {
                self.winners.insert(round, &round_winners);
            }

            self.round = self.round + 1;
            
            Ok(())
        }
       
        #[ink(message)]
        fn consume_quote(&mut self, _quote: Quote) -> IDNResult { Ok(()) }
       
        #[ink(message)]
        fn consume_sub_info(&mut self, _sub_info_response: SubInfoResponse) -> IDNResult { Ok(()) }
    }
}