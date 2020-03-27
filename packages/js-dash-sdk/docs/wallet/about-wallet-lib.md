### About Wallet-lib 

When Dash.Client is initiated with a `mnemonic` property, a wallet instance is automatically created accessible via `client.wallet` as well as an `client.account` instance. 

In order to ensure the sync-up with the network has happened, you will need to wait for the method `client.isReady()` to resolve. 

You will find else where the [complete documentation of Wallet-lib](https://github.com/dashevo/wallet-lib), as we only cover the most basic pieces in this documentation.
