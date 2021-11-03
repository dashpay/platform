**Usage**: `await client.core.getBlockByHash(hash, options)`  
**Description**: Allow to fetch a specific block by its hash

Parameters:

| parameters                | type                | required       | Description                                                                                      |
|---------------------------|---------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **hash**                  | String              | yes            | A valid block hash |
| **options**               | DAPIClientOptions   | no             |  |

Returns : {Promise<null|Buffer>} - The specified bufferized block
