# Timelock Sealed-Bid Auction

A basic example of using timelock encryption within a smart contract.

## How it Works

The tlock auction contract lets assets, created via the assets pallet, be auctioned for the chain's native token.

The contract provides several nice features:
- bids are sealed until the deadline through timelock encryption
- when the auction concludes, the winner receives the 

The keyword is *atomic*. The winner receives the asset only when they are able to submit their promised payment.

## Future Work

This work can be expanded to a more general design pattern for timelock encryption within a contract.