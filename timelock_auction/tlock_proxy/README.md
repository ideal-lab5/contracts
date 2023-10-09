# Proxy Contract

The proxy contract acts as a proxy between auctions and participants. It orchestrates auction participation by enforcing time-based rules for when auctions can be called. Additionally, it acts as a registry for NFTs which are bought and sold through the auction.

## Testing

### Unit  tests

``` bash
cargo test
```


### E2E tests

End-to-end tests reequires that you run a node locally and provide it's absolute path (e.g. /home/.../substrate/target/release/node-template). 

``` bash
export CONTRACTS_NODE="YOUR_CONTRACTS_NODE_PATH"
cargo +nightly test --features e2e-tests
```
