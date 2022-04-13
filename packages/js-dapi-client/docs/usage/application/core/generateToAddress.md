**Usage**: `await client.core.generateToAddress(blockMumber, address, options)`  
**Description**: Allow to broadcast a valid **signed** transaction to the network.
**Notes**: Will only works on regtest.

Parameters:

| parameters                | type                | required       | Description                                                                                      |
|---------------------------|---------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **blocksNumber**          | Number              | yes            | A number of block to see generated on the regtest network                                        |
| **address**               | String              | yes            | The address that will receive the newly generated Dash                                           |
| **options**               | DAPIClientOptions   | no             |  |

Returns : {Promise<string[]>} - a set of generated blockhashes.
