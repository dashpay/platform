**Usage**: `account.generateAddress(path)`    
**Description**: Generate an address from a path and import it to the store    
**Notes**: Usage of generate is discouraged, used `account.getAddress()` instead.    

Parameters:   

| parameters        | type   | required       | Description                                          |  
|-------------------|--------|----------------| -----------------------------------------------------|
| **path**          | String | yes            | BIP44 path of the address to generate                |

Returns : address object (metadata : `path, index, address, transactions, balanceSat, unconfirmedBalanceSat, utxos, fetchedLast, used`).
