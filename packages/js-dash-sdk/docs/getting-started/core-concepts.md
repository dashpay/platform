## Core concepts

The [Dash Core Developer Guide](https://dashcore.readme.io/docs/core-guide-introduction) will answer most of the questions about the fundamentals of Dash.   

However, some elements provided by the SDK need to be grasped, so we will quickly cover some of those.

## Wallet

At the core of Dash is the Payment Chain, in order to be able to transact on it, one needs to have a set of [UTXO](https://dashcore.readme.io/docs/core-guide-block-chain-transaction-data) that is controlled by a Wallet instance.  

In order to access your UTXO, you will have to provide a valid mnemonic that will unlock the Wallet and automatically fetch the associated UTXOs.

## Wallet accounts

Since the introduction of deterministic wallet, a Wallet is actually composed of multiple account. 

For manipulation account, `Wallet.getAccount()` takes an optional (default: 0) account Id. 

## Schema

The Dash Platform Application Chain, provides to developers the ability to create application.   
That application requires a set of rules and conditions describe in a portable document in the form of a JSON names : Application Schema. 

This SDK will use these schema in order to help you working with the platform in coordination with those set of rules.  

If you need to use more than just one specific schema, look up for how to use [multiples schemas](/getting-started/multiples-schemas.md)
