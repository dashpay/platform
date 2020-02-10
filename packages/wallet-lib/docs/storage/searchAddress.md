**Usage**: `storage.searchAddress(address, forceLoop)`      
**Description**: Returns a specific address information from the store    
**Notes**: We maintain mapped (cache) address for easy look-up, forceLoop value allow to outpass that cache and force a slow lookup in the store.    

Parameters: 

| parameters             | type              | required       | Description                                                            |  
|------------------------|-------------------|----------------| -----------------------------------------------------------------------|
| **address**            | String            | yes            | The Address identifier (Base 58 hash representation) to search for     |
| **forceLoop**          | Boolean           | no (def: false)| Used to bypass the cache and force looping over the addresses          |

Returns : Object ({found, result, type, address, walletId}).
