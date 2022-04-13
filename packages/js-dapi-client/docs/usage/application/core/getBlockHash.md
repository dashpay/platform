**Usage**: `await client.core.getBlockHash(height, options)`  
**Description**: Allow to fetch a specific block hash from  its height

Parameters:

| parameters                | type                | required       | Description                                                                                      |
|---------------------------|---------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **height**                | Number              | yes            | A valid block height |
| **options**               | DAPIClientOptions   | no             |  |

Returns : {Promise<null|string>} - the corresponding block hash
