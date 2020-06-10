const {
  TransactionsWithProofsRequest,
  TransactionsFilterStreamPromiseClient,
  BloomFilter: BloomFilterMessage,
} = require('@dashevo/dapi-grpc');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {subscribeToTransactionsWithProofs}
 */
function subscribeToTransactionsWithProofsFactory(grpcTransport) {
  /**
   * @typedef {subscribeToTransactionsWithProofs}
   * @param {object} bloomFilter
   * @param {Uint8Array|Array} bloomFilter.vData - The filter itself is simply a bit
   * field of arbitrary byte-aligned size. The maximum size is 36,000 bytes.
   * @param {number} bloomFilter.nHashFuncs - The number of hash functions to use in this filter.
   * The maximum value allowed in this field is 50.
   * @param {number} bloomFilter.nTweak - A random value to add to the seed value in the
   * hash function used by the bloom filter.
   * @param {number} bloomFilter.nFlags - A set of flags that control how matched items
   * are added to the filter.
   * @param {DAPIClientOptions & subscribeToTransactionsWithProofsOptions} [options]
   * @returns {
   *    EventEmitter|!grpc.web.ClientReadableStream<!TransactionsWithProofsResponse>
   * }
   */
  async function subscribeToTransactionsWithProofs(bloomFilter, options = { }) {
    // eslint-disable-next-line no-param-reassign
    options = {
      count: 0,
      // Override global timeout option
      // and timeout for this method by default
      timeout: undefined,
      ...options,
    };

    const bloomFilterMessage = new BloomFilterMessage();

    let { vData } = bloomFilter;

    if (Array.isArray(vData)) {
      vData = new Uint8Array(vData);
    }

    bloomFilterMessage.setVData(vData);
    bloomFilterMessage.setNHashFuncs(bloomFilter.nHashFuncs);
    bloomFilterMessage.setNTweak(bloomFilter.nTweak);
    bloomFilterMessage.setNFlags(bloomFilter.nFlags);

    const request = new TransactionsWithProofsRequest();
    request.setBloomFilter(bloomFilterMessage);

    if (options.fromBlockHeight) {
      request.setFromBlockHeight(options.fromBlockHeight);
    }

    if (options.fromBlockHash) {
      request.setFromBlockHash(
        Buffer.from(options.fromBlockHash, 'hex'),
      );
    }

    request.setCount(options.count);

    return grpcTransport.request(
      TransactionsFilterStreamPromiseClient,
      'subscribeToTransactionsWithProofs',
      request,
      options,
    );
  }

  return subscribeToTransactionsWithProofs;
}

/**
 * @typedef {object} subscribeToTransactionsWithProofsOptions
 * @property {string} [fromBlockHash] - Specifies block hash to start syncing from
 * @property {number} [fromBlockHeight] - Specifies block height to start syncing from
 * @property {number} [count=0] - Number of blocks to sync,
 *                                if set to 0 syncing is continuously sends new data as well
 */

module.exports = subscribeToTransactionsWithProofsFactory;
