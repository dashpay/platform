## About plugins

In order to add features and logic to the Wallet-library and be able to share independant module and request them together. 
Wallet-lib can be passed some plugins at his instantiation. 
Plugins are particular shaped class that can perform action on your wallet.

By default, three plugins are injected : BIP44Worker, SyncWorker and ChainWorker. 

They handle respectively with maintaining your address pool, getting you in sync with the blockchain and maintaining some knowledge about the chain (blockheight). 
You can disable them by adding `injectDefaultPlugins:false` at the initialization parameter of your wallet object.
 
For more granularity, you could do it as a parameter of `getAccount(accId, accOpts)`.

## Type of plugins 

There are three different types of plugins that can be used in the wallet-library:

- Workers : A worker plugins is a plugin that inherits from Worker class. It distinguish itself by having a execute method that will be executed each `workerIntervalTime`.
- Standard : These are mostly enhancers of the wallet library functionalities.

## Dependencies

In order for a plugin to have the ability to access wallet data, you have to add a dependency in the constructor.

```
class MyPlugin extends StandardPlugin { 
   constructor(){
    this.dependencies = ['walletId']
   }
   doStruff(){
     return this.walletId.substr(0);
   }
}
```

This will allow to access the walletId property; the same thing is doable with the account function.

## Accessing a plugin 


```
const account = wallet.getAccount(0);
account.getPlugin('pluginName');
```
