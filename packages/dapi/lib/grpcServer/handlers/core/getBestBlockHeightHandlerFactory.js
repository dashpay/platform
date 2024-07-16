const {
  v0: {
    GetBestBlockHeightResponse,
  },
} = require('@dashevo/dapi-grpc');

const logger = require('../../../logger');

/**
 * @param {CoreRpcClient} coreRPCClient
 * @param {ZmqClient} coreZmqClient
 * @returns {getBestBlockHeightHandler}
 */
function getBestBlockHeightHandlerFactory(coreRPCClient, coreZmqClient) {
  let height = null;

  // Reset height on a new block, so it will be obtained again on a user request
  coreZmqClient.on(
    coreZmqClient.topics.hashblock,
    () => {
      height = null;

      logger.trace({ endpoint: 'getBestBlockHeight' }, 'cleanup best block height cache');
    },
  );

  /**
   * @typedef getBestBlockHeightHandler
   * @return {Promise<GetBestBlockHeightResponse>}
   */
  async function getBestBlockHeightHandler() {
    if (height === null) {
      const start = Date.now();

      height = await coreRPCClient.getBestBlockHeight();

      const elapsedTime = Date.now() - start;

      logger.trace({
        endpoint: 'getBestBlockHeight',
      }, `cached best block height ${height}. took ${elapsedTime}ms`);
    }

    logger.trace({
      endpoint: 'getBestBlockHeight',
    }, `responded with cached best block height ${height}`);

    const response = new GetBestBlockHeightResponse();
    response.setHeight(height);

    return response;
  }

  return getBestBlockHeightHandler;
}

module.exports = getBestBlockHeightHandlerFactory;
