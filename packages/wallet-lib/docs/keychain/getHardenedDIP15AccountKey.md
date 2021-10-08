**Usage**: `keychain.getHardenedDIP15AccountKey(accountIndex = 0, type = 'HDPrivateKey')`    
**Description**: Return a safier root path to derivate from

Parameters: 

| parameters        | type        | required                  | Description                                                 |  
|-------------------|-------------|---------------------------| ------------------------------------------------------------|
| **accountIndex**  | number      | no (default:0)            | set the account index                                       |
| **type**          | string      | no (default:HDPrivateKey) | Enter a valid type (one of: ['HDPrivateKey','HDPublicKey']) |

Returns : HDPrivateKey (of path: `m/9'/1'/15'/accountIndex'` on testnet or `m/9'/5'/15'/accountIndex'` on livenet)

Example: 
```js

const hdPrivateKey = keychain.getHardenedDIP15AccountKey();
const { privateKey } = hdPrivateKey;

```
