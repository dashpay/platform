# Quick start

### ES5/ES6 via NPM

In order to use this library in Node, you will need to add it to your project as a dependency.

Having [NodeJS](https://nodejs.org/) installed, just type in your terminal : 

```sh
npm install @dashevo/wallet-lib
```

### CDN Standalone

For browser usage, you can also directly rely on unpkg for wallet-lib, and [localForage](https://github.com/localForage/localForage) as adapter for persistence.  

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
wallet.getAccount().then((account) => {
  // At this point, account has fetched all UTXOs if they exist
  const balance = account.getTotalBalance();
  console.log(`Balance: ${balance}`);

  // We easily can get a new address to fund
  const { address } = account.getUnusedAddress();
});
```

In above code, we did not specify any `transport` instance, as by default, wallet-lib is using DAPI as a transporter; The `adapter` not being set, we will use by default an in-memory (without persistence) adapter.    
One can set any adapter that contains a valid adapter syntax (getItem, setItem), such as [localForage](https://www.npmjs.com/package/localforage), you can learn more about [creating your own persistence adapter](develop/persistence.md).

Quick note :

- If no mnemonic is provided (nor any privatekey, HDPubKey,...), or if mnemonic is `null`, a mnemonic will be created for you automatically.  
- **By default, if not provided, network value will be `evonet`**.
- If no adapter specified, Wallet-lib will use an in-memory store (and warn you about it).
- If no transport specified, Wallet-lib will connect to DAPI.
- `wallet.getAccount()` is by default equivalent to `wallet.getAccount({ index:0 })`, where 0 correspond of the account index as per [BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki).

## Make a payment to an address

```js
const options = {
  recipient:'yLptqWxjgTxtwKJuLHoGY222NnoeqYuN8h',
  satoshis:100000
};
const transaction = account.createTransaction(options)
```

## Broadcast the transaction 

```js
const txid = await account.broadcastTransaction(transaction);
```

## Some rules of thumb

- There are multiple event listeners (socket sync,...), running intervals (service worker,...),
therefore a good way to quit an instance would be to call `account.disconnect()` which will care to
call `clearWorker(), closeSocket()` of the different elements. You can still decide to remove them by hand if you want.
- Some classic examples of usage can be seen here : [Examples](/usage/examples.md)
