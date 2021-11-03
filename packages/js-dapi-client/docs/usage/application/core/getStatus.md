**Usage**: `await client.core.getStatus(options)`  
**Description**: Allow to fetch a specific block hash from  its height

Parameters:

| parameters                | type                | required       | Description                                                                                      |
|---------------------------|---------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **options**               | DAPIClientOptions   | no             |  |

Returns : {Promise<object>} - Status object

```js
const status = await client.core.getStatus()
/**
{
  coreVersion: 150000,
  protocolVersion: 70216,
  blocks: 10630,
  timeOffset: 0,
  connections: 58,
  proxy: '',
  difficulty: 0.001745769130443678,
  testnet: false,
  relayFee: 0.00001,
  errors: '',
  network: 'testnet'
}
**/
```
