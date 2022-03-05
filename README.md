# Contracts

A collection of smart contracts used on the [iris blockchain](https://github.com/iridium-labs/substrate/tree/iris).

## Setup

Follow the [ink! documentation](https://paritytech.github.io/ink-docs/) for a complete guide on getting started.

To compile a wasm blob and metadata for a contract, navigate to the contract's root directory and run:

``` bash
cargo +nightly contract build
```

## Deployment

The simplest method to deploy contracts is to use the polkadotjs ui. After starting an Iris node, navigate to the contracts tab and follow the instructions [here](https://docs.substrate.io/tutorials/v3/ink-workshop/pt1/#creating-an-ink-project).

## Modules (contracts)

### Iris Asset Exchange

A decentralized marketplace for exchanging tokens for assets. That is, a marketplace for buying and selling access to and ownership of data.

### Composable Access Rules

Composable Access Rules is a set of contracts that data owners can use to configure additional business logic that must be executed before consumers can access data. These contracts execute when a consumer (token holder) requests data from the network. Rules include contracts such as a "single use" for an owned asset, or placing expiration dates on assets.

## Testing

``` bash
cargo +nightly test
```
