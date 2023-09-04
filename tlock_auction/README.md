# Timelock Message Vault

A basic example of using timelock encryption within a smart contract.

## How it Works

At this most superficial level, this contract stores an AES public key and exposes a function to allow participants to publish messages. The messages should be encrypted for the AES pubkey, but there is no check for this currently (in the future, we will need a proof). The contract also exposes a `reveal` function, allowing anyone with knowledge of the secret key to decrypt the messages and store the plaintexts in the contract. 