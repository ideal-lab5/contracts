# Timelock Commit-Reveal

This smart contract uses the ETF network to enable a timelocked commit-reveal.

The contract is deployed witha future `deadline`, a slot identity in the future.

`commit(timelocked_value, commitment)`
`reveal()`

## How it works

1. The contract deployer initiates the contract with a publicly known slot schedule that messages should be encrypted for.
    - ([sl1, ..., sl_k], t) along with an AES public key.
2. 

## Use Cases

- Sealed Bid Auction
// need to get current slot/block and ensure after deadline
// decrypt the timelocked txs and choose winner
// slash misbehaving winners -> just store in the contract for now. deposit goes to auctioneer? or redistribute to valid participants
// 
// if winner has too low funds => check next winner => repeat until winner
// 


- need to provide a min deposit to bid
- send tlocked tx along with commitment
- should lose deposit if your balance is too low

- initial version will be SUPER simple. Basically timelocked guess a number