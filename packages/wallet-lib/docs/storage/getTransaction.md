**Usage**: `storage.getTransaction(transactionId)`     
**Description**: Return the transaction from the store matching the txId.

Parameters: 

| parameters             | type              | required       | Description                                                             |  
|------------------------|-------------------|----------------| ------------------------------------------------------------------------|
| **transactionId**      | String            | yes            | The transaction id to fetch from the state                           |


Returns: TransactionObject     

Example: 

```js
storage.getTransaction('4f71db0c4bf3e2769a3ebd2162753b54b33028e3287e45f93c5c7df8bac5ec7e')
```
