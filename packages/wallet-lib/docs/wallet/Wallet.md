**Usage**: `new Wallet(walletOpts)`  
**Description**: This method creates a new Wallet.  
In Wallet-Lib, a Wallet is a manager that is tied to a passphrase/seed or privateKey and manage one or multiples Account from that.   
It's purpose is mainly to create or get an account, allowing multiple account to be tracked and tied from a single manager.    

Parameters: 

| parameters                               | type               | required           | Description                                                                                                                                                                    |  
|------------------------------------------|--------------------|--------------------| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **walletOpts.network**                   | string/Network     | no (def:'evonet')  | Use either a string reference to Networks ('livenet', 'testnet') or it's Networks representation                                                           |
| **walletOpts.mnemonic**                  | string/Mnemonic    | no                 | If sets at null, generate a new mnemonic. If sets to a valid value, create wallet from mnemonic                                                           |
| **walletOpts.passphrase**                | string             | no                 | If sets at null, generate a new privateKey. It sets to a valid privateKey, uses it (with the passphrase if provided) to unlock the seed                                                           |
| **walletOpts.offlineMode**               | boolean            | no (def: false)    | Set to true to not perform any request to the network |
| **walletOpts.injectDefaultPlugins**      | boolean            | no (def: true)     | Use to inject default plugins on loadup (BIP44Worker, ChainWorker and SyncWorker) |
| **walletOpts.allowSensitiveOperations**  | boolean            | no (def: false)    | If you want a special plugin to access the keychain or other sensitive operation, set this to true. |
| **walletOpts.cache.addresses**           | object             | no                 | If you have your cache state somewhere else (fs) you can fetch and pass it along for faster sync-up |
| **walletOpts.cache.transactions**        | object             | no                 | If you have your cache state somewhere else (fs) you can fetch and pass it along for faster sync-up |
| **walletOpts.plugins**                   | Array              | no                 | It you have some plugins, worker you want to pass to wallet-lib. You can pass them as constructor or initialized object  |
| **walletOpts.seed**                      | string             | no                 | If you only have a seed representation, you can pass it instead of mnemonic to init the wallet from it  |
| **walletOpts.HDPrivateKey**              | string/HDPrivateKey| no                 | If you only have a HDPrivateKey representation, you can pass it instead of mnemonic to init the wallet from it  |
| **walletOpts.HDPublicKey**               | string/HDPublicKey  | no                 | If you only have a HDPublicKey representation, you can pass it instead of mnemonic to init the wallet from it  |
| **walletOpts.privateKey**                | string/PrivateKey  | no                 | If you only have a PrivateKey representation, you can pass it instead of mnemonic to init the wallet from it  |


N.B 1 : If both mnemonic, seed and privateKey are filled, only mnemonic will be used. If none is entered, the wallet will create a mnemonic.
N.B 2 : When initialized from a `privateKey` or an `HDPublicKey`, comportment of Wallet-lib differs slightly. 

- PrivateKey : There is no path in this mode. It's a unique public address. 
- HDPublicKey : There is no signing in this mode. Watch-only.

Returns : Wallet instance.

**Examples** :

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
