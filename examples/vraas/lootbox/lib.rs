#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod lootbox {

    use idn_contracts::prelude::*;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    #[derive(Debug, PartialEq, Eq, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Reward {
        Bronze,  // 50% chance
        Silver,  // 30% chance
        Gold,    // 15% chance
        Diamond, // 5% chance
    }

    /// track user's reward counts
    #[derive(Debug, Default, PartialEq, Eq, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    pub struct RewardStats {
        bronze: u32,
        silver: u32,
        gold: u32,
        diamond: u32,
    }

    #[ink(storage)]
    pub struct Lootbox {
        idn_client: IdnClient,
        subscription_id: Option<SubscriptionId>,
        // track registered users for the next lootbox
        registered_users: Vec<AccountId>,
        // track if a user is already registered (to prevent duplicates)
        is_registered: Mapping<AccountId, bool>,
        // track rewards received by users
        user_rewards: Mapping<AccountId, RewardStats>,
        // the total number of pulses receivedc
        round: u32,
    }

    #[ink(event)]
    pub struct UserRegistered {
        #[ink(topic)]
        user: AccountId,
        total_registered: u32,
    }

    #[ink(event)]
    pub struct LootboxOpened {
        total_users: u32,
    }

    #[ink(event)]
    pub struct RewardGranted {
        #[ink(topic)]
        user: AccountId,
        reward: Reward,
    }

    impl Lootbox {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                idn_client: IdnClient::new(4502, 40, 4594, 16, 6, 10000000),
                subscription_id: None,
                registered_users: Vec::new(),
                is_registered: Mapping::new(),
                user_rewards: Mapping::new(),
                round: 0,
            }
        }

        #[ink(message)]
        pub fn create_subscription(&mut self) {
            self.subscription_id = Some(
                self.idn_client
                    .create_subscription(100, 10, None, None, None, None)
                    .unwrap(),
            );
        }

        /// Register to get a reward with the next dispatch
        #[ink(message)]
        pub fn register(&mut self) {
            let caller = self.env().caller();

            // Check if user is already registered
            if self.is_registered.get(caller).unwrap_or(false) {
                panic!("Already registered for this lootbox");
            }

            // Add user to registered list
            self.registered_users.push(caller);
            self.is_registered.insert(caller, &true);

            // Emit event
            self.env().emit_event(UserRegistered {
                user: caller,
                total_registered: u32::try_from(self.registered_users.len()).unwrap(),
            });
        }

        /// Process lootbox with random bytes (called by VRaaS callback)
        pub fn open_lootbox(&mut self, random_bytes: [u8; 32]) {
            let user_count = self.registered_users.len();
            assert!(user_count > 0, "No users registered");

            self.env().emit_event(LootboxOpened {
                total_users: u32::try_from(user_count).unwrap(),
            });

            // Distribute rewards to each registered user
            for (index, user) in self.registered_users.iter().enumerate() {
                // Use different bytes for each user to get unique randomness
                let user_random = self.get_user_random(&random_bytes, index);
                let reward = self.determine_reward(user_random);

                // Update reward stats
                let mut stats = self.user_rewards.get(user).unwrap_or_default();
                match reward {
                    Reward::Bronze => stats.bronze = stats.bronze.saturating_add(1),
                    Reward::Silver => stats.silver = stats.silver.saturating_add(1),
                    Reward::Gold => stats.gold = stats.gold.saturating_add(1),
                    Reward::Diamond => stats.diamond = stats.diamond.saturating_add(1),
                };

                self.user_rewards.insert(user, &stats);

                // Emit event
                self.env().emit_event(RewardGranted {
                    user: *user,
                    reward,
                });
            }

            // Clear registration for next lootbox
            self.clear_registrations();
        }

        /// Determine reward based on random value
        fn determine_reward(&self, random_value: u8) -> Reward {
            // Convert to 0-100 scale
            let roll = (u16::from(random_value).saturating_mul(100)).saturating_div(255);

            match roll {
                0..=49 => Reward::Bronze,    // 50%
                50..=79 => Reward::Silver,   // 30%
                80..=94 => Reward::Gold,     // 15%
                95..=100 => Reward::Diamond, // 5%
                _ => Reward::Bronze,
            }
        }

        /// Get unique random byte for each user
        fn get_user_random(&self, random_bytes: &[u8; 32], user_index: usize) -> u8 {
            // Combine multiple bytes for better distribution
            let idx = user_index % 32;
            let next_idx = (user_index.saturating_add(1)) % 32;

            // XOR two bytes for more randomness
            random_bytes[idx] ^ random_bytes[next_idx]
        }

        /// Clear all registrations
        fn clear_registrations(&mut self) {
            for user in self.registered_users.iter() {
                self.is_registered.remove(user);
            }
            self.registered_users.clear();
        }

        #[ink(message)]
        pub fn get_subscription_id(&self) -> Option<SubscriptionId> {
            self.subscription_id
        }

        #[ink(message)]
        pub fn get_round(&self) -> u32 {
            self.round
        }

        /// Get reward stats for a specific user
        #[ink(message)]
        pub fn get_user_rewards(&self, user: AccountId) -> RewardStats {
            self.user_rewards.get(user).unwrap_or_default()
        }

        /// Get reward stats for caller
        #[ink(message)]
        pub fn get_my_rewards(&self) -> RewardStats {
            let caller = self.env().caller();
            self.user_rewards.get(caller).unwrap_or_default()
        }

        /// Check if caller is registered
        #[ink(message)]
        pub fn is_user_registered(&self) -> bool {
            let caller = self.env().caller();
            self.is_registered.get(caller).unwrap_or(false)
        }

        /// Get all registered users (for testing/admin purposes)
        #[ink(message)]
        pub fn get_registered_users(&self) -> Vec<AccountId> {
            self.registered_users.clone()
        }
    }

    impl IdnConsumer for Lootbox {
        #[ink(message)]
        fn consume_pulse(
            &mut self,
            pulse: Pulse,
            _subscription_id: SubscriptionId,
        ) -> Result<(), Error> {
            let randomness = pulse.rand();
            self.round = self.round.saturating_add(1);
            self.open_lootbox(randomness);
            Ok(())
        }

        // Handle subscription quotes
        #[ink(message)]
        fn consume_quote(&mut self, _quote: Quote) -> Result<(), Error> {
            Ok(())
        }

        // Handle subscription information responses
        #[ink(message)]
        fn consume_sub_info(&mut self, _sub_info: SubInfoResponse) -> Result<(), Error> {
            Ok(())
        }
    }
}
