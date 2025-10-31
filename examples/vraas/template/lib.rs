#![cfg_attr(not(feature = "std"), no_std, no_main)]
use idn_contracts::prelude::*;

#[allow(clippy::arithmetic_side_effects)]
#[ink::contract]
mod coin_flip {
   
    use super::*;

    type Rand = <Pulse as TPulse>::Rand;
   
    #[ink(storage)]
    pub struct CoinFlip {
        idn_client: IdnClient,
        subscription_id: Option<SubscriptionId>,
        latest_rand: Option<Rand>,
    }
   
    impl CoinFlip {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                idn_client: IdnClient::new(4502, 40, 4594, 16, 6, 100000000),
                subscription_id: None,
                latest_rand: None,
            }
        }
       
        #[ink(message)]
        pub fn create_subscription(&mut self) {
            let sub_id = self.idn_client.create_subscription(10, 1, None, None, None, None).unwrap();
            self.subscription_id = Some(sub_id);
        }
        
        #[ink(message)]
        pub fn get_subscription_id(&self) -> Option<SubscriptionId> {
            self.subscription_id
        }
                        
        #[ink(message)]
        pub fn get_latest_rand(&self) -> Option<Rand> {
            self.latest_rand
        }
    }
   
    impl IdnConsumer for CoinFlip {
        #[ink(message)]
        fn consume_pulse(&mut self, pulse: Pulse, _subscription_id: SubscriptionId) -> IDNResult {
            let randomness = pulse.rand();
            self.latest_rand = Some(randomness);
            Ok(())
        }
       
        #[ink(message)]
        fn consume_quote(&mut self, _quote: Quote) -> IDNResult { Ok(()) }
       
        #[ink(message)]
        fn consume_sub_info(&mut self, _sub_info_response: SubInfoResponse) -> IDNResult { Ok(()) }
    }
}