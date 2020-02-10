**Usage**: `account.encrypt(method, data, secret)`    
**Description**: Allow to encrypt a value using a specific secret

Parameters:   

| parameters        | type   | required       | Description                                      |  
|-------------------|--------|----------------| -------------------------------------------------|
| **method**        | String | yes            | Enter a valid encryption method (one of: ['aes'])|
| **data**          | String | yes            | The value to encrypt                             |
| **secret**        | String | yes            | The secret used in order to encrypt the data     |

Returns : encrypted value (string).   

```js
const encrypted = account.encrypt('aes','coucou', 'secret');
console.log(encrypted);// U2FsdGVkX18+7ixRbZ7DzC8P8X/4ewNHSp2R6pZDmsI=
