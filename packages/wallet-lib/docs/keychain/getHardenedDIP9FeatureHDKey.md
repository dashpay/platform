**Usage**: `keychain.getHardenedDIP9FeatureHDKey(type)`    
**Description**: Return a safier root key to derivate from

Parameters: 

| parameters        | type        | required                  | Description                                                                                             |  
|-------------------|-------------|---------------------------| -------------------------------------------------|
| **type**          | string      | no (default:HDPrivateKey) | Enter a valid type (one of: ['HDPrivateKey','HDPublicKey']) |

Returns : HDPrivateKey (of path: `m/9'/1'` on testnet or `m/9'/5'` on livenet)

Example: 
```js

const hdPrivateKey = keychain.getHardenedDIP9FeatureHDKey();
const { privateKey } = hdPrivateKey;

```
