# Quick start

### ES5/ES6 via NPM

In order to use this library in Node, you will need to add it to your project as a dependency.

Having [NodeJS](https://nodejs.org/) installed, just type in your terminal : 

```sh
npm install @dashevo/wallet-lib
```

### CDN Standalone

For browser usage, you can also directly rely on unpkg for wallet-lib, and localforage as adapter for persistance.  

```
<script src="https://unpkg.com/@dashevo/wallet-lib"></script>
<script src="https://unpkg.com/localforage"></script>

<script>
const { Wallet } = require('@dashevo/wallet-lib');
const wallet = new Wallet({adapter: localforage});
</script>
```

## Initialization

Let's load our Wallet by creating a new Wallet instance specifying our mnemonic.  

```js
const { Wallet } = require('@dashevo/wallet-lib');

const opts = {
  network: 'testnet',
  mnemonic: "arena light cheap control apple buffalo indicate rare motor valid accident isolate",
};
const wallet = new Wallet(opts);
wallet.getAccount()
    .then((account)=>{
        console.log('Account ready to be used')
    });
```

In above code, we did not specify any `transport` instance, which by default, is equivalent to using DAPI as a transporter; The `adapter` being not set, we will use by default an in-memory (without persistance) adapter.    
One can set any adapter that contains a valid adapter syntax (getItem, setItem), such as [localforage](https://www.npmjs.com/package/localforage).

As you can see, we are waiting for the `ready` event to be thrown before using the Wallet.  
The purpose of this is to ensure that wallet-lib has pre-fetched all required elements (such as your UTXO set) and perform all required tasks (prefetch, sync, validate, workers exec,...) before starting playing with an account.  
Nothing force you to do so, this is mostly an helper provided to you.  


Quick note :
- If no mnemonic is provided (nor any privatekey, HDPubKey,...) or if mnemonic is `null`, a mnemonic will be created for you automatically.  
- **By default, if not provided, network value will be `evonet`**.
- If no adapter specified, Wallet-lib will use a in-memory store (and warn you about it).
- If no transport specified, Wallet-lib will connect to DAPI.
- `wallet.getAccount()` is by default equivalent to `wallet.getAccount(0)`, where 0 correspond of the account index as per [BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki).

## Make a payment to an address

```js
const options = {
  recipient:'yLptqWxjgTxtwKJuLHoGY222NnoeqYuN8h',
  satoshis:100000
};
account
  .createTransaction(options)
  .then((tx)=> console.log(tx));
```

## Broadcast the transaction 

```js
const txid = await account.broadcastTransaction(transaction);
```

## Some rules of thumb

- There is multiple event listeners(socker sync,...), running intervals (service worker,...),
therefore a good way to quit an instance would be to call `account.disconnect()` which will care to
call `clearWorker(), closeSocket()` of the differents elements. You can still decide to remove them by hand if you want.
- Some classic examples of usage can be seen here : [Examples](/usage/examples.md)
