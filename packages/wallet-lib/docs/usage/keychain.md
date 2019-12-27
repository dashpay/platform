## Keychain

This class handle the derivation and handling of the HDRootKey.  

The reason we keep the HDRootKey is because both the seed and the mnemonic would allow to generate other coins private keys, while a HDRootKey is specific to a coin. 

See below on how to generate keychain from seed or mnemonic.    

### Create a Keychain

```
const hdRootKey = 'tprv8ZgxMBicQKsPeWisxgPVWiXho8ozsAUqc3uvpAhBuoGvSTxqkxPZbTeG43mvgXn3iNfL3cBL1NmR4DaVoDBPMUXe1xeiLoc39jU9gRTVBd2';
const keychain = new KeyChain({HDPrivateKey:hdRootKey});
```

### Get keys for a specific path

```
const path = `m/44'/1'/0'/0/0`;
const {PrivateKey, PublicKey} = keychain.getKeyForPath(path);
```

### Generate Key For Path

The `getKeyForPath` method will handle it's own cache, therefore getKeyForPath might be returned from the cache.
If you really want to derivate, no matter of anything, this is the method to use.

```
const path = `m/44'/1'/0'/0/0`;
const {PrivateKey, PublicKey} = keychain.generateKeyForPath(path);
```

This method will not update the keychain cache.

### Create a keychain from mnemonic 

```js
const {KeyChain, utils} = require('@dashevo/wallet-lib');
const keychain = new KeyChain({ HDPrivateKey: utils.mnemonicToHDPrivateKey(mnemonic, 'testnet') });
```

### Create a keychain from seed 

```js
const {KeyChain, utils} = require('@dashevo/wallet-lib');
const keychain = new KeyChain({ HDPrivateKey: utils.seedToHDPrivateKey(seed, 'testnet') });
```

### Get an address from a HDPrivateKey 

```js 
    const {KeyChain} = require('@dashevo/wallet-lib');
   const {Address} = require('@dashevo/dashcore-lib');
   
   const hdRootKey = 'tprv8ZgxMBicQKsPeWisxgPVWiXho8ozsAUqc3uvpAhBuoGvSTxqkxPZbTeG43mvgXn3iNfL3cBL1NmR4DaVoDBPMUXe1xeiLoc39jU9gRTVBd2';
   const keychain = new KeyChain({HDPrivateKey:hdRootKey});
   
   const path = `m/44'/1'/0'/0/0`;
   const pubKey = keychain.getKeyForPath(path).publicKey.toAddress();
   const firstAccountFirstIndeAddress = new Address(pubKey).toString();
```
