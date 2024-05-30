const {
  v0: {
    GetBestBlockHeightRequest,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

/**
 *
 * @param {GrpcTransport} grpcTransport
 * @returns {getBestBlockHeight}
 */
function getBestBlockHeightFactory(grpcTransport) {
  /**
   * Returns block height of chain tip
   * @typedef {getBestBlockHeight}
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<string>}
   */
  async function getBestBlockHeight(options = {}) {
    const response = await grpcTransport.request(
      CorePromiseClient,
      'getBestBlockHeight',
      new GetBestBlockHeightRequest(),
      options,
    );

    return response.getHeight();
  }

  return getBestBlockHeight;
}

module.exports = getBestBlockHeightFactory;
