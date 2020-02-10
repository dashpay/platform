**Usage**: `storage.updateTransaction(transaction)`      
**Description**: Internally, this is mostly called to update the information of a transaction in the store. Works mostly more as an replace than an update.   

Parameters: 

| parameters             | type              | required       | Description                                               |  
|------------------------|-------------------|----------------| ----------------------------------------------------------|
| **transaction**        | TransactionObj    | yes            | The TransactionObject to update (uses tx.txid as key)     |

Returns : Boolean.

Example: 
```js
storage.updateTransaction({
  txid: '688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23',
  blockheight: 11212,
});
```

