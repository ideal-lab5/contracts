# Template

This is a template to get you started with building contracts that use verifiable randomness.

To build and test this contract, use the cargo contract tool.

## Building

``` sh
cargo +nightly contract build
```

## Deploy

``` shell
cargo contract instantiate myContract.contract --constructor new \
--args some args here \
--suri //Alice --url ws://127.0.0.1:9944 -x
```

## Testing

``` sh
cargo +nightly test
```