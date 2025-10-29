# About
This plinko game is a modified version of: https://github.com/mojtaba1180/plinko-game-react
The purpose of this game is to showcase how one can leverage the idn-contracts library to bring verifiable randomness into their ink! smart contracts on their own parachain.

# To Run
These instructions assume you have a compatible parachain and have read the docs regarding requirements. It also assumes that you have the IDN and your parachain running locally. 
Be sure in `contract/lib.rs` that you have the correct configurations when creating the IdnClient:
```
IdnClient::new(
    4502,          // IDN parachain ID
    40,            // IDN Manager pallet index
    4594,          // Your parachain ID
    16,            // Contracts pallet index on your chain
    6,             // Contract callback call index on your chain
    1_000_000_000, // Maximum XCM execution fees
)
````
## Contract
0. Ensure `cargo-contract` is installed
1. Navigate to the contract directory and run `cargo contract build --release`
2. Deploy the contract via your preferred method
3. Update `CONTRACT_ADDRESS` in `/src/context/PolkadotContext.tsx` with the new contract address

## UI
1. Ensure yarn is installed
2. Run `yarn build`
3. Run `yarn start`
4. Navigate to `http://localhost:3000/plinko`
5. Click Play and wait for your game to be played.