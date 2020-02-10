**Usage**: `account.decrypt(method, encryptedData, secret, encoding)`    
**Description**: Allow to decrypt an encrypted message

Parameters: 

| parameters        | type           | required       | Description                                                                                             |  
|-------------------|----------------|----------------| -----------------------------------------------------------|
| **method**        | String         | yes            | Enter a valid decrypt method (one of: ['aes'])             |
| **encryptedData** | String         | yes            | An encrypted value                                         |
| **secret**        | String         | yes            | The secret used for encrypting the data in first place     |
| **encoding**      | ['hex','utf8'] | no (def: utf8) | The secret used for encrypting the data in first place     |

Returns : decoded value (string).

```js
const decrypted = account.decrypt('aes','U2FsdGVkX18+7ixRbZ7DzC8P8X/4ewNHSp2R6pZDmsI=', 'secret')
console.log(decrypted);// coucou
```
