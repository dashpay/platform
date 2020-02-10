**Usage**: `account.getAddress(index, type)`    
**Description**: Get a specific address based on the index and type of address.  

Parameters:   

| parameters        | type   | required       | Description                                                                                 |  
|-------------------|--------|----------------| --------------------------------------------------------------------------------------------|
| **index**         | number | no             | Index of the address (starting at 0) - default:0                                            |
| **type**          | String | no             | Type of the address, one of ['external','internal','misc']. - default: external             |

Returns : address object (metadata : `path, index, address, transactions, balanceSat, unconfirmedBalanceSat, utxos, fetchedLast, used`).
