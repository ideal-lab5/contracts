# ETF Network Contract Toolkit

This is an ink! smart contract toolkit for building contracts on the ETF network. 

With the ETF network, contracts...

- have access to on-chain randomness to smart contracts
- can easily implement non-interactive multiparty protocols
- design contracts for use with **delayed transactions**

## ETF Network Environment

This environment is configured to allow ink! smart contracts to call the chain extension exposed by the ETF network runtime. This allows contracts to:
- read slot secrets from the chain (stored in the etf-Aura pallet) and use its presence to make time-based decisions in your smart contracts
- use the slot secrets as a source of randomness in smart contracts
  
  See the [bit-roulette](../examples/bit-roulette/) example for a demonstration.

### Configuration 

Add the dependency 
```
etf-contract-utils = { version = "0.1.0, git="https://github.com/ideal-lab5/contracts", default-features = false, features = ["ink-as-dependency"] }
```

``` rust
use etf_contract_utils::ext::EtfEnvironment;
#[ink::contract(env = EtfEnvironment)]
mod your_smart_contract {
    use crate::EtfEnvironment;
    ...
}
```

### Chain Extension Usage

``` rust
self.env()
    .extension()
    .secret(slot_number)
```

### Testing with the chain extension

To test functions that call the chain extension, it can be mocked with:

