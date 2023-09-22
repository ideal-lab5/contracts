# Contracts

[![Built with ink!](https://raw.githubusercontent.com/paritytech/ink/master/.images/badge.svg)](https://github.com/paritytech/ink)

## Setup

Follow the [ink! documentation](https://paritytech.github.io/ink-docs/getting-started/setup) for a complete guide on getting started.

To compile a wasm blob and metadata for a contract, navigate to the contract's root directory and run:

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

## Build

``` bash
cargo +nightly contract build
```

## Test

``` bash
cargo test 
```
