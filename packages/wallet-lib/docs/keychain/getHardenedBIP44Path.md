**Usage**: `keychain.getHardenedBIP44Path(type)`    
**Description**: Return a safier root path to derivate from

Parameters: 

| parameters        | type        | required                  | Description                                                                                             |  
|-------------------|-------------|---------------------------| -------------------------------------------------|
| **type**          | string      | no (default:HDPrivateKey) | Enter a valid type (one of: ['HDPrivateKey','HDPublicKey']) |

Returns : HDPrivateKey

Example: 
```js
const hdPrivateKey = keychain.getHardenedBIP44Path();
const { privateKey } = hdPrivateKey
```
