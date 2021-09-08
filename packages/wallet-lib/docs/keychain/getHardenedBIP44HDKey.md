**Usage**: `keychain.getHardenedBIP44HDKey(type)`    
**Description**: Return a safier root key to derivate from

Parameters: 

| parameters        | type        | required                  | Description                                                                                             |  
|-------------------|-------------|---------------------------| -------------------------------------------------|
| **type**          | string      | no (default:HDPrivateKey) | Enter a valid type (one of: ['HDPrivateKey','HDPublicKey']) |

Returns : HDPrivateKey

Example: 
```js
const hdPrivateKey = keychain.getHardenedBIP44HDKey();
const { privateKey } = hdPrivateKey
```
