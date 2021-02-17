**Usage**: `new Identities(wallet)`  
**Description**: This method creates a new Identities instance associated to the given wallet.   

Parameters: 

| parameters                                | type            | required           | Description                                                                                                                                                                    |  
|-------------------------------------------|-----------------|--------------------| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **wallet**                                | Wallet          | yes                | A valid [wallet](/wallet/Wallet) instance                                                                                                                                      |

Returns : Identities instance.

Examples (assuming a Wallet instance created) : 

```js
const { Identities, Wallet } = require('@dashevo/wallet-lib');
const wallet = new Wallet();
const identities = new Identities(wallet);
identities.getIdentityHDKeyByIndex(0, 0);
```
