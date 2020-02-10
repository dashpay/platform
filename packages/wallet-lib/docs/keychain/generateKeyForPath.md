**Usage**: `keychain.generateKeyForPath(path, type)`    
**Description**: Generate key for a specific path without storing the result in keychain. 
**Important**: For cache/perf reason, this method is discouraged in favor of getKeyForPath.

Parameters: 

| parameters        | type        | required                  | Description                                                 |  
|-------------------|-------------|---------------------------| ------------------------------------------------------------|
| **path**          | string      | yes                       | Enter a valid path                                          |
| **type**          | string      | no (default:HDPrivateKey) | Enter a valid type (one of: ['HDPrivateKey','HDPublicKey']) |

Returns : {HDPrivateKey|HDPublicKey}

Example: 
```js
const { privateKey } = keychain.generateKeyForPath(0);
```

ADD TESTS! PLUS PATH
