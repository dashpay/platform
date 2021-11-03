**Usage**: `await client.core.getMnListDiff(baseBlockHash, blockHash, options)`  
**Description**: Allow to fetch a specific block hash from  its height

Parameters:

| parameters                | type                | required       | Description                                                                                      |
|---------------------------|---------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **baseBlockHash**         | String              | yes            |  hash or height of start block |
| **blockHash**             | String              | yes            |  hash or height of end block |
| **options**               | DAPIClientOptions   | no             |  |

Returns : {Promise<object>} - The Masternode List Diff of the specified period
