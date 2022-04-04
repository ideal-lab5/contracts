# Contracts

A collection of smart contracts used on the [iris blockchain](https://github.com/iridium-labs/substrate/tree/iris).

## Setup

Follow the [ink! documentation](https://paritytech.github.io/ink-docs/getting-started/setup) for a complete guide on getting started.

To compile a wasm blob and metadata for a contract, navigate to the contract's root directory and run:

``` bash
cargo +nightly contract build
```

### Note on Binaryen/wasm-opt

If your package manager doesn't have binaryen versions >= 99, then:

- Download the latest version here: https://github.com/WebAssembly/binaryen/releases

- follow these instrutions to install:

``` bash
# unzip the tarball
sudo tar xzvf binaryen-version_100-x86_64-linux.tar.gz
# update permissions
chmod +x binaryen-version_100
# move to /opt
sudo mv binaryen-version_100 /opt/
# navigate to /opt
cd /opt
# make it executable
chmod +x binaryen-version_100
# add symbolic link to /usr/bin
sudo ln -s /opt/binaryen-version_100/bin/wasm-opt /usr/bin/wasm-opt
```

Verify the installation by running `wasm-opt --version`. If the command executes and the printed version matches the downloaded version, then the installation is complete.

## Build

``` bash
cargo +nightly contract build
```

## Test

``` bash
cargo test 
```

## Deployment

The simplest method to deploy contracts is to use the polkadot.js ui. After starting an Iris node, navigate to the contracts tab and follow the instructions [here](https://docs.substrate.io/tutorials/v3/ink-workshop/pt1/#creating-an-ink-project).

## Contracts

### Iris Asset Exchange

A decentralized marketplace for exchanging tokens for assets. That is, a marketplace for buying and selling access to and ownership of data.

### Composable Access Rules

Composable Access Rules is a set of contracts that data owners can use to configure additional business logic that must be executed before consumers can access data. These contracts execute when a consumer (token holder) requests data from the network. Rules include contracts such as a "single use" for an owned asset, or placing expiration dates on assets.
