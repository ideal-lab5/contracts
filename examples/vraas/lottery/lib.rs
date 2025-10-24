#![cfg_attr(not(feature = "std"), no_std, no_main)]
use idn_contracts::prelude::*;

#[ink::contract]
mod template {
    use super::*;
    use scale_info::prelude::vec::Vec;

    #[ink(storage)]
    pub struct MyContract {
        idn_client: IdnClient,
        subscription_id: Option<SubscriptionId>,
        // Lottery storage
        participants: Vec<AccountId>,
        ticket_price: Balance,
        lottery_active: bool,
        total_pot: Balance,
    }

    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum ContractError {
        GenericError,
        LotteryNotActive,
        LotteryAlreadyActive,
        InsufficientPayment,
        NoParticipants,
        TransferFailed,
    }

    #[ink(event)]
    pub struct RandomnessReceived {}

    #[ink(event)]
    pub struct TicketPurchased {
        #[ink(topic)]
        buyer: AccountId,
        ticket_number: u32,
    }

    #[ink(event)]
    pub struct LotteryStarted {
        ticket_price: Balance,
    }

    #[ink(event)]
    pub struct WinnerSelected {
        #[ink(topic)]
        winner: AccountId,
        prize: Balance,
    }

    #[ink(event)]
    pub struct LotteryEnded {
        participants: u32,
        total_pot: Balance,
    }

    impl MyContract {
        #[ink(constructor)]
        pub fn new(ticket_price: Balance) -> Self {
            Self {
                idn_client: IdnClient::new(
                    4502, // IDN parachain ID
                    40,   // IDN Manager pallet index
                    4594, // Your parachain ID
                    16,   // Contracts pallet index on your chain
                    6,    // Contract callback call index on your chain
                    1000000000, // Optional: Maximum XCM execution fees
                ),
                subscription_id: None,
                participants: Vec::new(),
                ticket_price,
                lottery_active: false,
                total_pot: 0,
            }
        }

        /// Start the lottery
        #[ink(message)]
        pub fn start_lottery(&mut self) -> Result<(), ContractError> {
            if self.lottery_active {
                return Err(ContractError::LotteryAlreadyActive);
            }

            self.lottery_active = true;
            self.participants.clear();
            self.total_pot = 0;

            // Create subscription through IDN client
            // 1000 credits, frequency = 10 (1 minute)
            let subscription_id = self
                .idn_client
                .create_subscription(1000, 10, None, None, None, None)
                .unwrap();

            // Update contract state with the new subscription
            self.subscription_id = Some(subscription_id);

            self.env().emit_event(LotteryStarted {
                ticket_price: self.ticket_price,
            });

            Ok(())
        }

        /// Buy a lottery ticket (payable)
        #[ink(message, payable)]
        pub fn buy_ticket(&mut self) -> Result<(), ContractError> {
            if !self.lottery_active {
                return Err(ContractError::LotteryNotActive);
            }

            let payment = self.env().transferred_value();
            if payment < self.ticket_price {
                return Err(ContractError::InsufficientPayment);
            }

            let buyer = self.env().caller();
            self.participants.push(buyer);
            self.total_pot = self.total_pot.saturating_add(payment);

            let ticket_number = u32::try_from(self.participants.len()).unwrap();

            self.env().emit_event(TicketPurchased {
                buyer,
                ticket_number,
            });

            Ok(())
        }

        /// Internal function to select winner and distribute prize
        fn select_winner_and_payout(&mut self, randomness: &[u8]) -> Result<(), ContractError> {
            if !self.lottery_active {
                return Ok(()); // No active lottery, nothing to do
            }

            if self.participants.is_empty() {
                // No participants, just end the lottery
                self.lottery_active = false;
                self.env().emit_event(LotteryEnded {
                    participants: 0,
                    total_pot: 0,
                });
                return Ok(());
            }

            // Convert randomness bytes to a number
            let mut random_value: u128 = 0;
            for (i, &byte) in randomness.iter().take(16).enumerate() {
                random_value |= (byte as u128) << (i.saturating_mul(8));
            }

            // Select winner
            #[allow(arithmetic_side_effects)]
            let winner_index = (random_value as usize).saturating_div(self.participants.len());
            let winner = self.participants[winner_index];

            let prize = self.total_pot;

            // Transfer pot to winner
            if self.env().transfer(winner, prize).is_err() {
                return Err(ContractError::TransferFailed);
            }

            self.env().emit_event(WinnerSelected { winner, prize });

            self.env().emit_event(LotteryEnded {
                participants: u32::try_from(self.participants.len()).unwrap(),
                total_pot: prize,
            });

            // Reset for next round
            self.lottery_active = false;
            self.participants.clear();
            self.total_pot = 0;

            Ok(())
        }

        // View functions
        #[ink(message)]
        pub fn get_participants(&self) -> Vec<AccountId> {
            self.participants.clone()
        }

        #[ink(message)]
        pub fn get_ticket_price(&self) -> Balance {
            self.ticket_price
        }

        #[ink(message)]
        pub fn get_total_pot(&self) -> Balance {
            self.total_pot
        }

        #[ink(message)]
        pub fn is_lottery_active(&self) -> bool {
            self.lottery_active
        }

        #[ink(message)]
        pub fn get_participant_count(&self) -> u32 {
            u32::try_from(self.participants.len()).unwrap()
        }
    }

    impl IdnConsumer for MyContract {
        #[ink(message)]
        fn consume_pulse(&mut self, pulse: Pulse, subscription_id: SubscriptionId) -> IDNResult {
            let randomness = pulse.rand();

            self.env().emit_event(RandomnessReceived {});

            // Automatically end lottery and select winner when randomness arrives
            self.select_winner_and_payout(&randomness).ok();

            Ok(())
        }

        #[ink(message)]
        fn consume_quote(&mut self, quote: Quote) -> IDNResult {
            Ok(())
        }

        #[ink(message)]
        fn consume_sub_info(&mut self, sub_info: SubInfoResponse) -> IDNResult {
            Ok(())
        }
    }
}
