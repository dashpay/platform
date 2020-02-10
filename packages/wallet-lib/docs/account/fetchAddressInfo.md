**Usage**: `account.fetchAddressInfo(addressObj, fetchUtxo)`    
**Description**: Fetch a specific address from the transport layer    
**Notes**: This method will have breaking changes with SPV implementation. We encourage you with using `Storage` or `DAPI-Client`.   

Parameters:   

| parameters        | type   | required       | Description                                          |  
|-------------------|--------|----------------| -----------------------------------------------------|
| **addressObj**    | String | yes            | Enter a valid encryption method (one of: ['aes'])    |
| **fetchUtxo**     | String | yes            | The value to encrypt (default: true)                 |

Returns : addrInfo (object representation of an address metadata)

N.B: An AddressObject is an intern representation consisting of a `{path, address, index}`  
