# Writing a new plugin

There is no control nor monitoring over third-party plugin. So anyone can write it's own plugin. 

In order for a plugin to have the ability to access wallet data, you have to add a dependency in the constructor.

Below, we create a Standard Plugin, see [Using a plugin](plugins/using-a-plugin.md) for more information about the different plugin types.

```js
const { StandardPlugin } = require('@dashevo/wallet-lib').plugins;

class MyWalletConsolidatorPlugin extends StandardPlugin { 
     constructor() {
        super({
          // When true, the wallet instance will only fire "ready" when a first execution of the plugin has happen.
          firstExecutionRequired: false,
          // Describe if we want to automatically execute it on starting up an account.
          executeOnStart: false,
          // Methods and function that we would want to use
          dependencies: [
            'getUTXOS',
            'getUnusedAddress',
            'getConfirmedBalance',
            'createTransactionFromUTXOS',
            'broadcastTransaction',
          ],
        });
      }

    consolidateWallet(address = this.getUnusedAddress().address, utxos = this.getUTXOS()) {
       return {
         prepareTransaction: () => {
           if (!utxos || utxos.length === 0) {
             throw new Error('There is nothing to consolidate');
           }
           const opts = {
             utxos,
             recipient: address,
           };
    
           const rawtx = this.createTransactionFromUTXOS(opts);
           return {
             toString: () => rawtx,
             broadcast: async () => {
               console.log(`BROADCASTING ${rawtx}`);
               return self.broadcastTransaction(rawtx);
             },
           };
         },
       };
     }
}
```

## Using my created plugin

When you plugin is created, including it in your Wallet is as easy as referencing up the class in the `plugins` array. 

```js 
const wallet = new Wallet({
    plugins:[MyWalletConsolidatorPlugin]
})
```

When some parameters are required first for your plugin to work, you might also decide to initialize first your plugin like this : 

```js 
const wallet = new Wallet({
    plugins:[new MyWalletConsolidatorPlugin({someOptions:true})]
});
```

## Accessing secure dependencies 

Due to the risk from running a plugin that have access to your keychain, these are, by default, not accessible.  
One would need to initialize a Wallet with the option `allowSensitiveOperations` set to `true`.  

You can see the list of thoses [sensitive functions and properties](https://github.com/dashevo/wallet-lib/blob/master/src/CONSTANTS.js#L67), anything under `UNSAFE_*` will require this option to be set to true in order to be use from within a plugin.  


## Accessing events 

From a plugin, you have the ability to listen to account's emitted events. 

```js
const { EVENT, plugins: { Worker } } = require('@dashevo/wallet-lib');
class NewBlockWorker extends Worker {
  constructor(options) {
    super({
      name: 'NewBlockWorker',
      executeOnStart: true,
      firstExecutionRequired: true,
      workerIntervalTime: 60 * 1000,
      gapLimit: 10,
      dependencies: [
        'storage',
        'transport',
        'walletId',
        'identities',
      ],
      ...options,
    });
  }

  async onStart() {
    this.parentEvents.on(EVENT.BLOCKHEIGHT_CHANGED, ({payload: blockHeight}) => {
      // on new blockheight do something.
    });
  }
}
```
