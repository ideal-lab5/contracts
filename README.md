# Ideal Labs Contracts Toolkit

[![Built with ink!](https://raw.githubusercontent.com/paritytech/ink/master/.images/badge.svg)](https://github.com/paritytech/ink)

Tools and examples for building ink! smart contracts that use publicly verifiable on-chain randomness.

## Usage

Follow the [ink! documentation](https://paritytech.github.io/ink-docs/getting-started/setup) for a complete guide on getting started.

To use this library, you must be running a node that supports:
- arkworks host functions
- the drand bridge pallet
- ink! smart contracts

You can find an example node [here](https://github.com/ideal-lab5/pallet-drand/tree/main/substrate-node-template).

> All contracts under the examples folder are outdated and under construction.
<!-- Checkout the [examples](./examples/) to get started. The [template](./template/) can be cloned as a jumping off point for new contracts. -->

### Configuration

To use in a smart contract, at `idl-contract-extension` to the cargo.toml
```toml
[dependencies]
idl-contract-extension = { git = "https://github.com/ideal-lab5/contracts.git", default-features = false, features = ["ink-as-dependency"] }

[features]
std = [
    ...
    "idl-contract-extension/std",
]
```

and configure the contract environment to use the `DrandEnvironment`

``` rust
use idl_contract_extension::ext::DrandEnvironment;
#[ink::contract(env = DrandEnvironment)]
mod your_smart_contract {
    use crate::DrandEnvironment;
    ...
}
```

#### Chain Extension

``` rust
self.env()
    .extension()
    .random();
```

### Build

```
cargo +nightly contract build
```

### Testing

#### Unit Tests
Unit tests can be run with

``` rust
cargo +nightly test
```

#### E2E tests

End-to-end tests reequires that you run a node locally and provide it's absolute path (e.g. /home/.../substrate/target/release/node-template). 

``` rust
export CONTRACTS_NODE="YOUR_CONTRACTS_NODE_PATH"
cargo +nightly test --features e2e-tests
```


### Note on Binaryen/wasm-opt

If your package manager doesn't have binaryen versions >= 99, then:

- Download the latest version here: https://github.com/WebAssembly/binaryen/releases

- follow these instrutions to install:

``` bash
# unzip the tarball
sudo tar xzvf binaryezn-version_100-x86_64-linux.tar.gz
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
