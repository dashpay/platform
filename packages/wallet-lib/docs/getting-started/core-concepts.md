## Core concepts

The [Dash Core Developer Guide](https://dashcore.readme.io/docs/core-guide-introduction) will answer most of the questions about the fundamentals of Dash.   

However, some elements provided by the SDK need to be grasped, so we will quickly cover some of those.

## Wallet

At the core of Dash is the Payment Chain, in order to be able to transact on it, one needs to have a set of [UTXO](https://dashcore.readme.io/docs/core-guide-block-chain-transaction-data) that is controlled by a Wallet instance.  

In order to access your UTXO, you will have to provide a valid mnemonic that will unlock the Wallet and automatically fetch the associated UTXOs.

## Wallet accounts

Since the introduction of deterministic wallet, a Wallet is actually composed of multiple account. 

For manipulation account, `Wallet.getAccount()` takes an optional (default: 0) account Id. 
