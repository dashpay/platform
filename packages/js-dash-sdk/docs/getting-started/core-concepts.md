## Core concepts

The [Dash Core Developer Guide](https://dashcore.readme.io/docs/core-guide-introduction) will answer most of questions about the fundamentals of Dash. However, some elements provided by the SDK need to be grasped, so we will quickly cover some of those.

## Wallet

At the core of Dash is the Payment Chain. In order to be able to transact on it, one needs to have a set of [UTXOs](https://dashcore.readme.io/docs/core-guide-block-chain-transaction-data) that are controlled by a Wallet instance.

In order to access your UTXO, you will have to provide a valid mnemonic that will unlock the Wallet and automatically fetch the associated UTXOs.

When an SDK instance is created, you can access your wallet via the `client.wallet` variable. (Check [wallet-lib documentation](https://dashevo.github.io/platform/Wallet-library/) for more details)

## Account

Since the introduction of deterministic wallets ([BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)), a wallet is represented by multiple accounts.

It is the instance you will use most of the time for receiving or broadcasting payments.

You can access your account with `client.getWalletAccount()`. See [how to use a different account](../examples/use-different-account.md) if you need to get an account at a specific index.

## App Schema and Contracts

The Dash Platform Chain provides developers with the ability to create applications. 
Each application requires a set of rules and conditions described as a portable document in the form of a JSON Schema.

When registered, those applications schemas are called contracts and contains a contractId (namespace : `client.platform.contracts`).  
By default, this library supports Dash Platform Name Service (DPNS) (to attach a name to an identity), under the namespace `client.platform.names` for testnet.  

See: [how to use multiple apps](../getting-started/multiple-apps.md)
