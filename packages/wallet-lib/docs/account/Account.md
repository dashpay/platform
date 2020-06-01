**Usage**: `new Account(wallet, accountOpts)`  
**Description**: This method creates a new Account associated to the given wallet.   
**Notes**: As it is directly linked to a wallet, you might want to rely on `Wallet.getAccount(index)` instead.     
When `wallet.offlineMode:true`, you can manage utxos / addresses via a cache options (or after init via the Storage controller).

Parameters: 

| parameters                                | type            | required       | Description                                                                                                                                                                    |  
|-------------------------------------------|-----------------|----------------| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **wallet**                                | Wallet          | yes            | A valid [wallet](/wallet/Wallet) instance                                                                                                                                      |
| **accountOpts.index**                     | number          | no             | The BIP44 account index; by default use the next one (n+1) of the biggest account index already created in wallet                                                              |
| **accountOpts.strategy**                  | string/function | no             | A valid strategy string identifier (amongst "simpleAscendingAccumulator", "simpleDescendingAccumulator", simpleTransactionOptimizedAccumulator") or your own strategy function |
| **accountOpts.label**                     | string          | no (def: null) | If you want to be able to reference to an account per label |
| **accountOpts.injectDefaultPlugins**      | boolean         | no (def: true) | Use to inject default plugins on loadup (BIP44Worker, ChainWorker and SyncWorker) |
| **accountOpts.allowSensitiveOperations**  | boolean         | no (def: false)| If you want a special plugin to access the keychain or other sensitive operation, set this to true. |
| **accountOpts.cacheTx**                   | boolean         | no (def: true) | If you want to cache the transaction internally (for faster sync-up) |
| **accountOpts.cache.addresses**           | object          | no             | If you have your addresses state somewhere else (fs) you can fetch and pass it along for faster sync-up |
| **accountOpts.cache.transactions**        | object          | no             | If you have your tx state somewhere else (fs) you can fetch and pass it along for faster sync-up |

Returns : Account instance.

Examples (assuming a Wallet instance created) : 

```js
const { Account, Wallet } = require('@dashevo/wallet-lib');
const wallet = new Wallet();
const account = new Account(wallet, {index: 42});
await account.init();
```
