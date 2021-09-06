**Usage**: `categorizeTransactions()`    
**Description**: Return for a transaction, the fee value that were used   

Parameters: 

| parameters        | type          | required       | Description                                      |  
|-------------------|---------------|----------------| -------------------------------------------------|
| **transactionsWithMetadata**   | [TransactionMetadata]   | yes            | Transaction with their metadata                           |
| **accountStore**   | Object   | yes            | Account store with addresses                           |
| **accountIndex**   | Number   | yes            | The account index                           |
| **walletType**   | WALLET_TYPES   | yes            | The wallet type                           |
| **network**   | Network/String   | no (def: testnet)            | Wallet network                           |

Returns : {[CategorizedTransaction]} - Array of categorized transactions 

```js
const categorizedTransactions = categorizeTransaction(transactionsWithMetadata, accountstore, 0, WALLET_TYPES.HDWALLET);
[{
    transaction: Transaction(),
    type: 'received',
    from: [{}],
    to: [{}],
    blockHash: '00001'
    height: 42,
    isInstantLocked: true,
    isChainLocked: true
}]
```