**Usage**: `account.broadcastTransaction(transaction)`      
**Description**: Allow to broadcast a valid **signed** transaction to the network.  
**Notes**: Requires a signed transaction, use [`account.sign(transaction)`](/account/sign) for that.  

Parameters: 

| parameters                        | type               | required       | Description                                                                                      |  
|-----------------------------------|--------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **transaction**                   | Transaction/String | yes            | A valid [created transaction](/account/createTransaction) or it's hexadecimal raw representation |
| **options**                       | Object             | no             | |
| **options.skipFeeValidation**     | Boolean            | no             | When set to true, and min relay fee is not met, will still try to broadcast a transaction        |

Returns : transactionId (string).

N.B : The TransactionID provided is subject to [transaction malleability](https://dashcore.readme.io/docs/core-guide-transactions-transaction-malleability), and is not a source of truth (the transaction might be included in a block with a different txid).
