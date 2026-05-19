import dapiGrpc from '@dashevo/dapi-grpc';

const {
  v0: {
    GetBestBlockHeightRequest,
    CorePromiseClient,
  },
} = dapiGrpc;

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

export default getBestBlockHeightFactory;
