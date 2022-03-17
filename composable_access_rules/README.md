# Composable Access Rules

This is a set of smart contracts that data owners can associate with their owned asset classes and which enable custom rules that data consumers (who hold an asset minted from the asset class) must follow in order to access the data associated with the asset class.

In general, these contracts will:

1. Check some condition
2. If the condition is met, the consumer's asset is *burned*
3. If the condition is **not** met, then the consumer can proceed to access the data.

The set of contracts described here are the official Iridium composable access rules. These contracts all meet strict security and testing guidelines and will be made available to users and developers on the Iris blockchain.

Since there is not a predictable way to exhaust all potential data access scenarios, data owners and developers also reserve the right to develop their own unique access rules and deploy them to Iris.

## Usage

These contract are generally used from within Iris itself. When data owners create a new asset class, they can associate a static set of contracts with their asset class. This results in a new contract being constructed. That is, each of the "composable access rules" exist at the asset id level.

## Limited Use Token

The `limited use token` contract stipulates that a token can only be used a preconfigured number of times. If usage attempts exceeds this number, the asset is burned.

## Perishable Token

This contract allows a data owner to set a 'use by' date on their token. If access is attempted after the data, then the token is burned.
