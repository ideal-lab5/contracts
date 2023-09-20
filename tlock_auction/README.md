# Timelock Auction

The **timelock auction contract** is a [Vickrey auction](https://en.wikipedia.org/wiki/Vickrey_auction), or `sealed-bid second-price auction`, enabled with timelock encryption via the ETF network. In a Vickrey auction, the highest bidder wins but the price paid is the second highest bid. Using timelock encryption enables a **non-interactive winner selection** for the auction, where all bids can be revealed with no interaction from the accounts that proposed them.

## Setup

To use the contract, it must be deployed to the ETF network. In addition, it should be communicated with via an app capable of encryption and decrypting data with the etf.js or etf-sdk libs. An example which uses the contract can be found [here]()

### Build

```
cargo +nightly contract build
```

### Testing

```
cargo test
```
