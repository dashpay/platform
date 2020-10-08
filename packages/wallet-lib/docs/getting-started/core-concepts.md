# Core concepts

The [Dash Core Developer Guide](https://dashcore.readme.io/docs/core-guide-introduction) will answer most of the questions about the fundamentals of Dash.   

However, some elements provided by the SDK need to be grasped, so we will quickly cover some of those.

## Wallet

At the core of Dash is the Payment Chain, in order to be able to transact on it, one needs to have a set of [UTXO](https://dashcore.readme.io/docs/core-guide-block-chain-transaction-data) that is controlled by a Wallet instance.  

In order to access your UTXO, you will have to provide a valid mnemonic that will unlock the Wallet and automatically fetch the associated UTXOs.

## Wallet accounts

Since the introduction of [deterministic wallet](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki), a Wallet is actually composed of multiple account. 

For manipulating multiple accounts, `Wallet.getAccount()` takes optional [options](/wallet/getAccount.md) where index allows either accessing or creating a specific account index. 

## Instantiation types

A Wallet instance can be created from multiples types, which impact how much the Wallet can do. 
In general, we expect you to initialize from a `mnemonic` or an `seed` (HD seed) or an `HDPrivateKey`, which allows wallet-lib to deal with HD Wallet (deterministic wallet).  

In some other cases, you might want to instantiate Wallet from another input such as : 
- `privateKey`: This allows managing a single privateKey/publicKey set. Therefore, you will only have a single unique address to receive money. 
- `HDPublicKey`: This allows a "watch-only" mode. You won't be able to spend anything, but this will allow you to track and monitor in real-time the address set of this public key. This allows you to derive unique unused addresses from a single key (shared by another user, third-party merchant).

[Learn more about instantiation documentation](/wallet/Wallet.md)
