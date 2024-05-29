const {
  v0: {
    GetBestBlockHeightResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {CoreRpcClient} coreRPCClient
 * @param {ZmqClient} coreZmqClient
 * @returns {getBestBlockHeightHandler}
 */
function getBestBlockHeightHandlerFactory(coreRPCClient, coreZmqClient) {
  let height = null;

  // Reset height on a new block, so it will be obtain again on a user request
  coreZmqClient.on(
    coreZmqClient.topics.hashblock,
    () => {
      height = null;
    },
  );

  /**
   * @typedef getBestBlockHeightHandler
   * @return {Promise<GetBestBlockHeightResponse>}
   */
  async function getBestBlockHeightHandler() {
    if (height === null) {
      height = await coreRPCClient.getBestBlockHeight();
    }

    const response = new GetBestBlockHeightResponse();
    response.setHeight(height);

    return response;
  }

  return getBestBlockHeightHandler;
}

module.exports = getBestBlockHeightHandlerFactory;
