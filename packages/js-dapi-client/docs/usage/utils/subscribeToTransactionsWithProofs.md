**Usage**: `async client.subscribeToTransactionsWithProofs(bloomFilter, options = { count: 0 })`
**Description**:

Parameters:

| parameters                | type             | required       | Description                                                                                             |
|----------------------------|------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **bloomFilter.vData**      | Uint8Array/Array | yes            | The filter itself is simply a bit field of arbitrary byte-aligned size. The maximum size is 36,000 bytes. |
| **bloomFilter.nHashFuncs** | Number           | yes            | The number of hash functions to use in this filter. The maximum value allowed in this field is 50. |
| **bloomFilter.nTweak**     | Number           | yes            | A random value to add to the seed value in the hash function used by the bloom filter. |
| **bloomFilter.nFlags**     | Number           | yes            | A set of flags that control how matched items are added to the filter. |
| **options.fromBlockHash**  | String           | yes            | Specifies block hash to start syncing from |
| **options.fromBlockHeight**| String           | yes            | Specifies block height to start syncing from |
| **options.count**          | Number           | no (default: 0)| Number of blocks to sync, if set to 0 syncing is continuously sends new data as well |

Returns : Promise<EventEmitter>|!grpc.web.ClientReadableStream<!TransactionsWithProofsResponse>

