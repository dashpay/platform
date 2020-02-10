**Usage**: `account.fetchTransactionInfo()`    
**Description**: Fetch a specific transaction from the transport layer    
**Notes**: This method will have breaking changes with SPV implementation. We encourage you with using `Storage` or `DAPI-Client`.   

Parameters:   

| parameters        | type   | required       | Description                                          |  
|-------------------|--------|----------------| -----------------------------------------------------|
| **transactionId** | String | yes            | identifier of the Transaction to retrieve            |

Returns : transaction object (metadata : `txid, blockhash, blockheight, blocktime, fees, size, vout, vin, txlock`).
