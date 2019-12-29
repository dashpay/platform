## Wallet

In Wallet-Lib, a Wallet is a manager that is tied to a passphrase/seed or privateKey and manage one or multiples accounts from that. 
It's purpose is mainly to create or get an account, allowing multiple account to be tracked and tied from a single manager.  

### Create a wallet

```
const { Wallet } = require('@dashevo/wallet-lib');
const DAPIClient = require('@dashevo/dapi-client');
const { Mnemonic } = require('@dashevo/dashcore-lib');
const localForage = require('localforage');


const mnemonic = 'my mnemonic in 12 or 24 words;'; //Can also be an instance of Mnemonic
const network = 'testnet' // or 'livenet'
const transport = new DAPIClient();
const adapter = localForage.createInstance({
      name: 'persist:walletAdapter'
    });

const opts = {
    transport, mnemonic, network, adapter
};
const Wallet = new Wallet(opts);
```
##### options

> **allowSensitiveOperations** : Bool(def: false) : When set at true, allow plugins to access storage or sensible information. 

> **injectDefaultPlugins** : Bool(def: true) : When set at false, disable default plugins (SyncWorker, ChainWorker, BIP44Worker)

> **passphrase** : String(def: null) : Set the passphrase to a mnemonic

> **mnemonic** : String|Mnemonic|null : When set will init the wallet from it. When null generates a new mnemonic.

> **HDPrivateKey** : String|HDPrivateKey : When set will init the wallet from it.

> **seed** : String : When set will init the wallet from it.

> **privateKey** : String|PrivateKey : When set will init the wallet from it.

> **HDPublicKey** : String|HDPublicKey : When set will init the wallet from it.

> **network** : String(def: testnet) : Will be used to set the network for various crypto manipulation (genAddr)

> **plugins** : Array(def: []) : Allow to pass specific plugins to a wallet (see Plugins documentation).

> **offlineMode** : Bool(false) : When set at true, the wallet will not try to use any transport layer.

N.B : If mnemonic, seed and privateKey are filled, only mnemonic will be used. If none is entered, the wallet will create a mnemonic.

### Creation without a mnemonic (gets one generated)
```js
const wallet = new Wallet();
```
or 
```js
const wallet = new Wallet({
  mnemonic: null
});
console.log(wallet.exportWallet());
```

### Creation from Mnemonic 

```js
const wallet = new Wallet({
  mnemonic: 'hole lesson insane entire dolphin scissors game dwarf polar ethics drip math'
})
```

### Creation from HDPrivateKey 

```js
const wallet = new Wallet({
  HDPrivateKey: 'tprv8ZgxMBicQKsPeWisxgPVWiXho8ozsAUqc3uvpAhBuoGvSTxqkxPZbTeG43mvgXn3iNfL3cBL1NmR4DaVoDBPMUXe1xeiLoc39jU9gRTVBd2'
})
```

### Creation from Seed 

```js
const wallet = new Wallet({
  seed: '436905e6756c24551bffaebe97d0ebd51b2fa027e838c18d45767bd833b02a80a1dd55728635b54f2b1dbed5963f4155e160ee1e96e2d67f7e8ac28557d87d96'
})
```

## Create an Account

```
const account = wallet.createAccount([opts]);
```

For the account options, see the Account documentation.
You also probably mean to use get an account instead. This is designed mostly to be Private as get an account deal with it. 

---

## Get an Account

```
const account = wallet.getAccount({index:0});

const accountOpts = {index:42, ... };
const account = wallet.getAccount(accountOpts); //If no account on index 42, will create with other passed options
```

##### params

> **accountOpts** : Object: If the account doesn't exist yet, we create it passing these options
> **accountOpts.index ** : Bool(def : 0) : If the account doesn't exist yet, we create it passing these options

See above `Create an Account` section for other options.

---

## Export a wallet

```
const HDPrivateKey = wallet.exportWallet(true);
const mnemonic = wallet.exportWallet();
```

##### params

> **toHDPrivateKey** : Bool(def : false) 

---

## Disconnect

```
wallet.disconnect();
```
