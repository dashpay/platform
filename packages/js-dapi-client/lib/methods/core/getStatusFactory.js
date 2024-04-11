const {
  v0: {
    GetStatusRequest,
    GetStatusResponse,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getStatus}
 */
function getStatusFactory(grpcTransport) {
  /**
   * Get Core chain status
   * @typedef {getStatus}
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<object>}
   */
  async function getStatus(options = {}) {
    const getStatusRequest = new GetStatusRequest();

    const response = await grpcTransport.request(
      CorePromiseClient,
      'getStatus',
      getStatusRequest,
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

    responseObject.status = Object.keys(GetStatusResponse.Status)
      .find((key) => GetStatusResponse.Status[key] === responseObject.status);

    return responseObject;
  }

  return getStatus;
}

module.exports = getStatusFactory;
