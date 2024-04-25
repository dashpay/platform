const {
  v0: {
    GetBlockchainStatusRequest,
    GetBlockchainStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getBlockchainStatus}
 */
function getBlockchainStatusFactory(grpcTransport) {
  /**
   * Get Core chain status
   * @typedef {getBlockchainStatus}
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<object>}
   */
  async function getBlockchainStatus(options = {}) {
    const getBlockchainStatusRequest = new GetBlockchainStatusRequest();

    const response = await grpcTransport.request(
      CorePromiseClient,
      'getBlockchainStatus',
      getBlockchainStatusRequest,
      options,
    );

    const responseObject = response.toObject();

    // Respond with Buffers instead of base64 for binary fields

    if (response.getChain()) {
      if (response.getChain()
        .getBestBlockHash()) {
        responseObject.chain.bestBlockHash = Buffer.from(response.getChain()
          .getBestBlockHash());
      }

      if (response.getChain()
        .getChainWork()) {
        responseObject.chain.chainWork = Buffer.from(response.getChain()
          .getChainWork());
      }
    }

    // Respond with constant names instead of constant values

    responseObject.status = Object.keys(GetBlockchainStatusResponse.Status)
      .find((key) => GetBlockchainStatusResponse.Status[key] === responseObject.status);

    return responseObject;
  }

  return getBlockchainStatus;
}

module.exports = getBlockchainStatusFactory;
