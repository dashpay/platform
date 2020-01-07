## Core concepts

The [Dash Core Developer Guide](https://dashcore.readme.io/docs/core-guide-introduction) will answer most of the questions about the fundamentals of Dash.   

However, some elements provided by the SDK need to be grasped, so we will quickly cover some of those.

## Wallet

At the core of Dash is the Payment Chain, in order to be able to transact on it, one needs to have a set of [UTXO](https://dashcore.readme.io/docs/core-guide-block-chain-transaction-data) that is controlled by a Wallet instance.  

In order to access your UTXO, you will have to provide a valid mnemonic that will unlock the Wallet and automatically fetch the associated UTXOs.

When a SDK instance is created, you can access your wallet via the `sdk.wallet` variable, with the [wallet-lib Wallet doc](https://dashevo.github.io/wallet-lib/#/usage/wallet)

## Account

Since the introduction of deterministic wallet ([BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)), a Wallet is a representation of multiple account. 

It is the instance you will use most of the time for receiving or broadcasting payments. 

You can access your account with `sdk.account` and see [how to use a different account](/examples/use-different-account) if you need to get a specific account index.

## App Schema and Contracts

The Dash Platform Application Chain, provides to developers the ability to create application.   
That application requires a set of rules and conditions describe in a portable document in the form of a JSON names : Application Schema.

When registered, those app schema are called contracts and contains a contractId (namespace : `sdk.contracts`).  
By default, DashJS supports DPNS (to attach a name to an identity), under the namespace `sdk.names` for evonet.  

You can read more on [how to use DPNS on a local evonet](/examples/use-local-evonet.md) or [how to use multiple apps](/getting-started/multiple-apps.md)
