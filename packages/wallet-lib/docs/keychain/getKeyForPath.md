**Usage**: `keychain.getKeyForPath(path,type)`    
**Description**: Get a key from the keychain cache or generate if not yet existing 

Parameters: 

| parameters        | type        | required                  | Description                                                                                             |  
|-------------------|-------------|---------------------------| -------------------------------------------------|
| **path**          | string      | yes                       | Enter a valid derivation path |
| **type**          | string      | no (default:HDPrivateKey) | Enter a valid type (one of: ['HDPrivateKey','HDPublicKey']) |

Returns : HDPrivateKey

Example: 
```js
const { privateKey } = keychain.getKeyForPath(`m/44'/1'/0'/0'/0`);
```
