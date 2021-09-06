**Usage**: `calculateTransactionFees(transaction)`    
**Description**: Return for a transaction, the fee value that were used   
**Notes**: To calculate the fee, provided transaction's input require output knowledge to be supplied       

Parameters: 

| parameters        | type          | required       | Description                                      |  
|-------------------|---------------|----------------| -------------------------------------------------|
| **transaction**   | Transaction   | yes            | A transaction instance                           |

Returns : {number} - fee value used for this transaction 