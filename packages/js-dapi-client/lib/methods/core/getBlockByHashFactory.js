const {
  v0: {
    GetBlockRequest,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getBlockByHash}
 */
function getBlockByHashFactory(grpcTransport) {
  /**
   * Get block by hash
   * @typedef {getBlockByHash}
   * @param {string} hash
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<null|Uint8Array>}
   */
  async function getBlockByHash(hash, options = {}) {
    const getBlockRequest = new GetBlockRequest();
    getBlockRequest.setHash(hash);

    const response = await grpcTransport.request(
      CorePromiseClient,
      'getBlock',
      getBlockRequest,
      options,
    );
    const blockBinaryArray = response.getBlock();

    return new Uint8Array(blockBinaryArray);
  }

  return getBlockByHash;
}

module.exports = getBlockByHashFactory;
