#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod society {
    use ink::storage::Mapping;
    use dkg_core::*;
    // not a big fan of having to do this here...
    use ark_bls12_381::{
        Bls12_381, Fr,
        G1Projective as G1, G2Affine, 
        G2Projective as G2
    };
    use ark_ec::{
        AffineRepr, CurveGroup,
        pairing::Pairing,
    };
    use ark_ff::UniformRand;
    use ark_serialize::CanonicalDeserialize;

    #[derive(scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct CiphertextVerification {
        pub u: [u8; 32],
        pub w: [u8; 32],
    }

    #[ink(storage)]
    pub struct Society {
        // the id of a society
        society_id: [u8;32],
        // the 'file system'
        fs: Mapping<[u8;32], CiphertextVerification>,
    }

    impl Society {
        #[ink(constructor, payable)]
        pub fn new(society_id: [u8; 32]) -> Self {
            let fs = Mapping::default();
            Self { society_id, fs }
            // Self { society_id }
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default())
        }

        /// this is a little weird but not sure of an alternative
        /// the hash is the computed hash_h(u, &v) where &v is the ciphertext
        /// since we can't pass an unbounded length input here
        /// we have to enforce that this has is computer offchain.
        /// we can also use the hash as an identifier
        /// 
        /// Each param should be encoded as a BigUint
        /// 
        #[ink(message)]
        pub fn publish(
            &mut self, 
            g: [u8;32], 
            u: [u8;32], 
            hash: [u8;32], 
            w: [u8;32]
        ) {
            // get caller id
            // verify caller is part of the society
            // then encode in storage
            let g1G1 = G1::deserialize_compressed(&g[..]).unwrap();
            let uG1 = G1::deserialize_compressed(&u[..]).unwrap();
            let hG2 = G2::deserialize_compressed(&hash[..]).unwrap();
            let wG2 = G2::deserialize_compressed(&w[..]).unwrap();
            let is_valid = dkg::verify_ciphertext(g1G1, uG1, hG2, wG2);
            if is_valid {
                self.fs.insert(hash, &CiphertextVerification { u, w });
            }
        }

        // /// Simply returns the current value of our `bool`.
        // #[ink(message)]
        // pub fn get(&self) -> bool {
        //     self.value
        // }
    }
}

    // / Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    // / module and test functions are marked with a `#[test]` attribute.
    // / The below code is technically just normal Rust code.
//     #[cfg(test)]
//     mod tests {
//         /// Imports all the definitions from the outer scope so we can use them here.
//         use super::*;

//         /// We test if the default constructor does its job.
//         #[ink::test]
//         fn default_works() {
//             let society = Society::default();
//             assert_eq!(society.get(), false);
//         }

//         /// We test a simple use case of our contract.
//         #[ink::test]
//         fn it_works() {
//             let mut society = Society::new(false);
//             assert_eq!(society.get(), false);
//             society.flip();
//             assert_eq!(society.get(), true);
//         }
//     }


//     /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
//     ///
//     /// When running these you need to make sure that you:
//     /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
//     /// - Are running a Substrate node which contains `pallet-contracts` in the background
//     #[cfg(all(test, feature = "e2e-tests"))]
//     mod e2e_tests {
//         /// Imports all the definitions from the outer scope so we can use them here.
//         use super::*;

//         /// A helper function used for calling contract messages.
//         use ink_e2e::build_message;

//         /// The End-to-End test `Result` type.
//         type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

//         /// We test that we can upload and instantiate the contract using its default constructor.
//         #[ink_e2e::test]
//         async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
//             // Given
//             let constructor = SocietyRef::default();

//             // When
//             let contract_account_id = client
//                 .instantiate("society", &ink_e2e::alice(), constructor, 0, None)
//                 .await
//                 .expect("instantiate failed")
//                 .account_id;

//             // Then
//             let get = build_message::<SocietyRef>(contract_account_id.clone())
//                 .call(|society| society.get());
//             let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
//             assert!(matches!(get_result.return_value(), false));

//             Ok(())
//         }

//         /// We test that we can read and write a value from the on-chain contract contract.
//         #[ink_e2e::test]
//         async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
//             // Given
//             let constructor = SocietyRef::new(false);
//             let contract_account_id = client
//                 .instantiate("society", &ink_e2e::bob(), constructor, 0, None)
//                 .await
//                 .expect("instantiate failed")
//                 .account_id;

//             let get = build_message::<SocietyRef>(contract_account_id.clone())
//                 .call(|society| society.get());
//             let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
//             assert!(matches!(get_result.return_value(), false));

//             // When
//             let flip = build_message::<SocietyRef>(contract_account_id.clone())
//                 .call(|society| society.flip());
//             let _flip_result = client
//                 .call(&ink_e2e::bob(), flip, 0, None)
//                 .await
//                 .expect("flip failed");

//             // Then
//             let get = build_message::<SocietyRef>(contract_account_id.clone())
//                 .call(|society| society.get());
//             let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
//             assert!(matches!(get_result.return_value(), true));

//             Ok(())
//         }
//     }
// }
