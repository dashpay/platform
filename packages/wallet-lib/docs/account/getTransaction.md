**Usage**: `account.getTransaction(txid)`      
**Description**: This method will return the transaction of a specific id

Parameters: 

| parameters  | type      | required       | Description                                                                       |  
|-------------|-----------|----------------| -------------------------------------------------------------------------------	  |
| **txid**    | string    | yes            | TxId of the transaction to fetch. |

Return : transaction with metadata

```js
account.getTransaction('1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6');
/*
  returns : 
  {
    transaction: Transaction,
    metadata: {
      blockHash: '0000007a84abfe1d2b4201f4844bb1e59f24daf965c928281589269f281abc01',
      height: 551438,
      instantLocked: true,
      chainLocked: true
    }
  }
 */
```
