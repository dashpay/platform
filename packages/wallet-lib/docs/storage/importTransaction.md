**Usage**: `storage.importTransaction(transaction, metadata?)`     
**Description**: Allow to import a transaction to the store.    
**Notes**: TransactionObject needs to contains basic vin/vout information.

Parameters: 

| parameters             | type               | required       | Description                                                             |  
|------------------------|--------------------|----------------| ------------------------------------------------------------------------|
| **transaction**        | Object/Transaction | yes            | The transaction to import to the store                                  |
| **metadata**           | TransactionMetaData| no             | The transaction metadata                                                |


Returns: Boolean     
Emits: `FETCHED_CONFIRMED_TRANSACTION`/`FETCHED_UNCONFIRMED_TRANSACTION`
