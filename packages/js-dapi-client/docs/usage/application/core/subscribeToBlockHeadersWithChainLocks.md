**Usage**: `await client.core.subscribeToBlockHeadersWithChainLocks(options = { count: 0 })`\
**Description**: Returns a ClientReadableStream streaming of block headers and chainlocks.


Parameters:

| parameters                | type             | required       | Description                                                                                             |
|----------------------------|------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **options.fromBlockHash**  | String           | yes            | Specifies block hash to start syncing from |
| **options.fromBlockHeight**| Number           | yes            | Specifies block height to start syncing from |
| **options.count**          | Number           | no (default: 0)| Number of blocks to sync, if set to 0 syncing is continuously sends new data as well |

Returns : Promise<EventEmitter>|!grpc.web.ClientReadableStream<!BlockHeadersWithChainLocksResponse>

Example :

```js
const { BlockHeader, ChainLock } = require('@dashevo/dashcore-lib');

const stream = await client.subscribeToBlockHeadersWithChainLocks({ fromBlockHeight: 0 });

stream
      .on('data', (response) => {
        const rawHeaders = response.getBlockHeaders();
        const rawChainLock = response.getChainLock();

        if (headers.length > 0) {
          const headers = rawHeaders.map((rawHeader) =>  new BlockHeader(rawHeader));
          console.dir(headers);
        }

        if (rawChainLock) {
          const chainLock = new ChainLock(rawChainLock);
        }
      })
    .on('error', (err) => {
        // do something with err
      });
```
