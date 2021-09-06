**Usage**: `filterTransactions(accountStore, walletType, accountIndex, transactions)`    
**Description**: 

Parameters: 

| parameters        | type          | required       | Description                                      |  
|-------------------|---------------|----------------| -------------------------------------------------|
| **addressStore**  | Object   | yes            | Account store with addresses                           |
| **accountIndex**  | Number   | yes            | The account index                           |
| **walletType**    | WALLET_TYPES   | yes            | The wallet type                           |

Returns : {[Transaction]} - Array of transaction filtered 

```js
filterTransactions(accountStore, walletType, accountIndex, transactions);
[Transaction,...];
```