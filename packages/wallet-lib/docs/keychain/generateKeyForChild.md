**Usage**: `keychain.generateKeyForChild(index,type)`    
**Description**: Generate key for a specific index without storing the result in keychain. 
**Important**: For cache/perf reason, this method is discouraged in favor of getKeyForPath.

Parameters: 

| parameters        | type        | required                  | Description                                      |  
|-------------------|-------------|---------------------------| -------------------------------------------------|
| **index**         | number      | yes                       | Enter a valid index to derivate to               |
| **type**          | string      | no (default:HDPrivateKey) | Enter a valid type (one of: ['HDPrivateKey','HDPublicKey']) |

Returns : {HDPrivateKey|HDPublicKey}

Example: 
```js
const { privateKey } = keychain.generateKeyForChild(0);
```


VERIFY THAT IT ACTUALLY WORKS AS EXPECTED LOL.
