**Usage**: `extendTransactionsWithMetadata(addressStore, accountIndex, walletType)`    
**Description**: Return for an addressStore, accountIndex and wallet type, the array classified set of addresses

Parameters: 

| parameters        | type          | required       | Description                                      |  
|-------------------|---------------|----------------| -------------------------------------------------|
| **addressStore**  | Object   | yes            | Account store with addresses                           |
| **accountIndex**  | Number   | yes            | The account index                           |
| **walletType**    | WALLET_TYPES   | yes            | The wallet type                           |

Returns : {[TransactionsWithMetadata]} - Array of transactions with metadata addresses 

```js
extendTransactionsWithMetadata(transactions, transactionsMetadata);
[
  [
    Transaction,
    { 
        blockHash: '0000012cf6377c6cf2b317a4deed46573c09f04f6880dca731cc9ccea6691e19',
        height: 555508,
        instantLocked: true,
        chainLocked: true
    }
  ]
];
```