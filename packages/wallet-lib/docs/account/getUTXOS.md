**Usage**: `account.getUTXOS(onyAvailable)`      
**Description**: This method will return the list of all available UTXOS for this account.

Parameters: 

| parameters           | type      | required       | Description                                                                             |  
|----------------------|-----------|----------------| ----------------------------------------------------------------------------------------|
| **onlyAvailable**    | boolean   | no (def: true) | When set at true, returns only the UTXOS that are available for use (spendable outputs) |

Returns : Array[utxos].
