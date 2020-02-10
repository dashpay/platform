**Usage**: `keychain.sign(transaction, privateKeys, sigType)`    
**Description**: Allow to sign a transaction with private keys

Parameters: 

| parameters        | type        | required       | Description                                                                                             |  
|-------------------|-------------|----------------| -------------------------------------------------|
| **object**        | Transaction | yes            | Enter a valid encryption method (one of: ['aes']) |
| **privateKeys**   | PrivateKey  | yes            | The private keys used to sign                             |
| **sigtype**       | String      | no             | Default: crypto.Signature.SIGHASH_ALL     |

Returns : Signed Transaction.
