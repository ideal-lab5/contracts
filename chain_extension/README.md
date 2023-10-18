# ETF Network Environment

This environment is configured to allow ink! smart contracts to call the chain extension exposed by the ETF network runtime.

## Configuration 

Add the dependency 
```
etf-chain-extension = { version = "0.1.0, default-features = false, features = ["ink-as-dependency"] }
```

``` rust
use etf_chain_extension::ext::EtfEnvironment;
#[ink::contract(env = EtfEnvironment)]
mod your_smart_contract {
    use crate::EtfEnvironment;
    ...
}
```


## Usage

``` rust
self.env()
    .extension()
    .check_slot(deadline)
```
