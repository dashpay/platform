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

> **mnemonic** : String|Mnemonic : When set will init the wallet from it.

> **seed** : String|HDPrivateKey : When set will init the wallet from it.

> **privateKey** : String|PrivateKey : When set will init the wallet from it.

> **HDExtPublicKey** : String|HDPublicKey : When set will init the wallet from it.

> **network** : String(def: testnet) : Will be used to set the network for various crypto manipulation (genAddr)

> **plugins** : Array(def: []) : Allow to pass specific plugins to a wallet (see Plugins documentation).

> **offlineMode** : Bool(false) : When set at true, the wallet will not try to use any transport layer.

N.B : If mnemonic, seed and privateKey are filled, only mnemonic will be used. If none is entered, the wallet will create a mnemonic.

## Create an Account

```
const account = wallet.createAccount([opts]);
```

For the account options, see the Account documentation.

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
