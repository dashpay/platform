**Usage**: `keychain.getKeyForChild(index,type)`    
**Description**: Use to derivate the root key to a specific child. (useful when Wallet is initialized from a HDPublicKey)

Parameters: 

| parameters        | type        | required                  | Description                                                                                             |  
|-------------------|-------------|---------------------------| -------------------------------------------------|
| **index**         | number      | yes                       | Enter a valid index |
| **type**          | string      | no (default:HDPublicKey) | Enter a valid type (one of: ['HDPrivateKey','HDPublicKey']) |

Returns : HDPrivateKey

Example: 
```js
const { privateKey } = keychain.getKeyForChild(0);
```
