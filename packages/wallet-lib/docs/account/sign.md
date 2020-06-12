**Usage**: `account.sign(transaction, privateKeys, sigType)`    
**Description**: Allow to sign a transaction with private keys   
**Notes**: A Signable Object is of type : Transaction or Message (exported by DashJS).   

Parameters: 

| parameters        | type        | required       | Description                                                                                             |  
|-------------------|-------------|----------------| -------------------------------------------------|
| **object**        | Signable    | yes            | Enter a valid encryption method (one of: ['aes'])|
| **privateKeys**   | PrivateKey  | yes            | The private keys used to sign                    |
| **sigtype**       | String      | no             | Default: crypto.Signature.SIGHASH_ALL            |

Returns : Signed Signable Object.

## Examples

### Signing a transaction
```js
const tx = account.createTransaction();
const signedTx = account.sign(tx); // Will find the privateKey from keychain for you. 
```

### Signing a message 
```js
const {Message} = require('dash');
const message = new Message('hello, world');

const idPrivateKey = account.getIdentityHDKeyByIndex(0, 0).privateKey;

const signed = account.sign(message, idPrivateKey);
const verify = message.verify(idPrivateKey.toAddress().toString(), signed.toString()); // true
```
