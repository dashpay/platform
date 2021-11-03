**Usage**: `await client.core.getBlockByHeight(height, options)`  
**Description**: Allow to fetch a specific block by its height

Parameters:

| parameters                | type                | required       | Description                                                                                      |
|---------------------------|---------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **height**                | Number              | yes            | A valid block height |
| **options**               | DAPIClientOptions   | no             |  |

Returns : {Promise<null|Buffer>} - The specified bufferized block
