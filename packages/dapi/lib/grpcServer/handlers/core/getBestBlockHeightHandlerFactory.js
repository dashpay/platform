const {
  v0: {
    GetBestBlockHeightResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {CoreRpcClient} coreRPCClient
 * @returns {getBestBlockHeightHandler}
 */
function getBestBlockHeightHandlerFactory(coreRPCClient) {
  /**
   * @typedef getBestBlockHeightHandler
   * @return {Promise<GetBestBlockHeightResponse>}
   */
  async function getBestBlockHeightHandler() {
    const height = await coreRPCClient.getBestBlockHeight();

    const response = new GetBestBlockHeightResponse();
    response.setHeight(height);

    return response;
  }

  return getBestBlockHeightHandler;
}

module.exports = getBestBlockHeightHandlerFactory;
