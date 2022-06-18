# Composable Access Rules

Composable Access Rules allows data owners to implement custom logic that data consumers are beholden to when fetching their data.

## Usage

Each composable access rule must implement the [ComposableAccessRule trait](./composable_access_rule.rs).
To build a composable access rule, each contract must implement the execute function:

`fn execute(&mut self, asset_id: u32, consumer: ink_env::AccountId) -> bool`

## Building

`cargo +nightly contract build`

## Testing

`cargo +nightly test`
