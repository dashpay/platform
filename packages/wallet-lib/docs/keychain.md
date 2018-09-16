## Keychain

This class handle the derivation and handle the HDRootKey.

### Create a Keychain

```
const keychain = new KeyChain(hdRootKey);
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