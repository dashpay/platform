**Usage**: `async client.subscribeToTransactionsWithProofs(bloomFilter, options = { count: 0 })`\
**Description**: For any provided bloomfilter, it will return a ClientReadableStream streaming the transaction matching the filter.


Parameters:

| parameters                | type             | required       | Description                                                                                             |
|----------------------------|------------------|----------------| ------------------------------------------------------------------------------------------------ |
| **bloomFilter.vData**      | Uint8Array/Array | yes            | The filter itself is simply a bit field of arbitrary byte-aligned size. The maximum size is 36,000 bytes. |
| **bloomFilter.nHashFuncs** | Number           | yes            | The number of hash functions to use in this filter. The maximum value allowed in this field is 50. |
| **bloomFilter.nTweak**     | Number           | yes            | A random value to add to the seed value in the hash function used by the bloom filter. |
| **bloomFilter.nFlags**     | Number           | yes            | A set of flags that control how matched items are added to the filter. |
| **options.fromBlockHash**  | String           | yes            | Specifies block hash to start syncing from |
| **options.fromBlockHeight**| Number           | yes            | Specifies block height to start syncing from |
| **options.count**          | Number           | no (default: 0)| Number of blocks to sync, if set to 0 syncing is continuously sends new data as well |

Returns : Promise<EventEmitter>|!grpc.web.ClientReadableStream<!TransactionsWithProofsResponse>

Example :

```js
const filter; // A BloomFilter object
const stream = await client.subscribeToTransactionsWithProofs(filter, { fromBlockHeight: 0 });

stream
      .on('data', (response) => {
        const merkleBlock = response.getRawMerkleBlock();
        const transactions = response.getRawTransactions();

        if (merkleBlock) {
          const merkleBlockHex = Buffer.from(merkleBlock).toString('hex');
        }

        if (transactions) {
          transactions.getTransactionsList()
              .forEach((tx) => {
                  // tx are probabilistic, so you will have to verify it's yours
                  const tx = new Transaction(Buffer.from(tx));
              });
        }
      })
    .on('error', (err) => {
        // do something with err
      });
```
