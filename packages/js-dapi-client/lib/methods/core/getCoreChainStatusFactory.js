const {
  v0: {
    GetCoreChainStatusRequest,
    GetCoreChainStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getCoreChainStatus}
 */
function getCoreChainStatusFactory(grpcTransport) {
  /**
   * Get Core chain status
   * @typedef {getCoreChainStatus}
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<object>}
   */
  async function getCoreChainStatus(options = {}) {
    const getCoreChainStatusRequest = new GetCoreChainStatusRequest();

    const response = await grpcTransport.request(
      CorePromiseClient,
      'getCoreChainStatus',
      getCoreChainStatusRequest,
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

    responseObject.status = Object.keys(GetCoreChainStatusResponse.Status)
      .find((key) => GetCoreChainStatusResponse.Status[key] === responseObject.status);

    return responseObject;
  }

  return getCoreChainStatus;
}

module.exports = getCoreChainStatusFactory;
