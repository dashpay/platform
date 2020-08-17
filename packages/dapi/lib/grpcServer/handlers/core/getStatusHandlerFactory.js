const {
  v0: {
    GetStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {InsightAPI} insightAPI
 * @returns {getStatusHandler}
 */
function getStatusHandlerFactory(insightAPI) {
  /**
   * @typedef getStatusHandler
   * @return {Promise<GetStatusResponse>}
   */
  async function getStatusHandler() {
    const { info } = await insightAPI.getStatus('getInfo');

    const {
      version,
      protocolversion,
      blocks,
      timeoffset,
      connections,
      proxy,
      difficulty,
      testnet,
      relayfee,
      errors,
      network,
    } = info;

    const response = new GetStatusResponse();
    response.setCoreVersion(version);
    response.setProtocolVersion(protocolversion);
    response.setBlocks(blocks);
    response.setTimeOffset(timeoffset);
    response.setConnections(connections);
    response.setProxy(proxy);
    response.setDifficulty(difficulty);
    response.setTestnet(testnet);
    response.setRelayFee(relayfee);
    response.setErrors(errors);
    response.setNetwork(network);

    return response;
  }

  return getStatusHandler;
}

module.exports = getStatusHandlerFactory;
