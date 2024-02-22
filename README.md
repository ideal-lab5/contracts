# ETF Contracts Toolkit

[![Built with ink!](https://raw.githubusercontent.com/paritytech/ink/master/.images/badge.svg)](https://github.com/paritytech/ink)

Tools for building smart contracts on the [ETF network](https://etf.idealabs.network).

The ETF consensus mechanism enables:

- on-chain randomness for smart contracts
- dapps based on delayed transactions

## Usage

Follow the [ink! documentation](https://paritytech.github.io/ink-docs/getting-started/setup) for a complete guide on getting started.


Contracts built with this library will only work when deployed to the ETF network. Deployment instructions follow standard ink! contract deployment instruction. The easiest way to deploy your contract is with the [cargo contract](https://github.com/paritytech/cargo-contract) tool:

``` shell
cargo contract instantiate myContract.contract --constructor new \
--args some args here \
--suri //Alice --url ws://127.0.0.1:9944 -x
```

### Configuration

To use in a smart contract, at `etf-contract-utils` to the cargo.toml
```toml
[dependencies]
etf-contract-utils = { git = "https://github.com/ideal-lab5/contracts", default-features = false, features = ["ink-as-dependency"] }

[features]
std = [
    ...
    "etf-contract-utils/std",
]
```

and configure the contract environment to use the `EtfEnvironment`

``` rust
use etf_contract_utils::ext::EtfEnvironment;
#[ink::contract(env = EtfEnvironment)]
mod your_smart_contract {
    use crate::EtfEnvironment;
    ...
}
```

#### Chain Extension

``` rust
self.env()
    .extension()
    .secret();
```


Checkout the [examples](./examples/) to get started. The [template](./template/) can be cloned as a jumping off point for new contracts.

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

##### Testing with the chain extension

To test functions that call the chain extension, it can be mocked like:

``` rust
struct MockETFExtension;
impl ink_env::test::ChainExtension for MockETFExtension {
    fn func_id(&self) -> u32 {
        1101
    }

    fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
        let mut ret = [1;48];
        ret[0] = 0;
        scale::Encode::encode_to(&ret, output);
        0
    }
}

ink_env::test::register_chain_extension(MockETFExtension);
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
