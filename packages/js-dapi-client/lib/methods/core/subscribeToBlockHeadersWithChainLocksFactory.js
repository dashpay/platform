const {
  v0: {
    BlockHeadersWithChainLocksRequest,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const DAPIClientError = require('../../errors/DAPIClientError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {subscribeToBlockHeadersWithChainLocks}
 */
function subscribeToBlockHeadersWithChainLocksFactory(grpcTransport) {
  /**
   * @typedef {subscribeToBlockHeadersWithChainLocks}
   * @param {DAPIClientOptions & subscribeToBlockHeadersWithChainLocksOptions} [options]
   * @returns {
   *    EventEmitter|!grpc.web.ClientReadableStream<!BlockHeadersWithChainLocksResponse>
   * }
   */
  async function subscribeToBlockHeadersWithChainLocks(options = { }) {
    // eslint-disable-next-line no-param-reassign
    options = {
      count: 0,
      // Override global timeout option
      // and timeout for this method by default
      timeout: undefined,
      ...options,
    };

    if (options.fromBlockHeight === 0) {
      throw new DAPIClientError('Invalid argument: minimum value for `fromBlockHeight` is 1');
    }

    const request = new BlockHeadersWithChainLocksRequest();

    if (options.fromBlockHeight !== undefined) {
      request.setFromBlockHeight(options.fromBlockHeight);
    }

    if (options.fromBlockHash) {
      request.setFromBlockHash(
        Buffer.from(options.fromBlockHash, 'hex'),
      );
    }

    request.setCount(options.count);

    return grpcTransport.request(
      CorePromiseClient,
      'subscribeToBlockHeadersWithChainLocks',
      request,
      options,
    );
  }

  return subscribeToBlockHeadersWithChainLocks;
}

/**
 * @typedef {object} subscribeToBlockHeadersWithChainLocksOptions
 * @property {string} [fromBlockHash] - Specifies block hash to start syncing from
 * @property {number} [fromBlockHeight] - Specifies block height to start syncing from
 * @property {number} [count=0] - Number of blocks to sync,
 *                                if set to 0 syncing is continuously sends new data as well
 */

module.exports = subscribeToBlockHeadersWithChainLocksFactory;
