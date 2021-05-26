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
   *
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
      if (response.getChain().getBestBlockHash()) {
        responseObject.chain.bestBlockHash = Buffer.from(response.getChain().getBestBlockHash());
      }

      if (response.getChain().getChainWork()) {
        responseObject.chain.chainWork = Buffer.from(response.getChain().getChainWork());
      }
    }

    if (response.getMasternode()) {
      if (response.getMasternode().getProTxHash()) {
        responseObject.masternode.proTxHash = Buffer.from(response.getMasternode().getProTxHash());
      }
    }

    // Respond with constant names instead of constant values

    responseObject.status = Object.keys(GetStatusResponse.Status)
      .find((key) => GetStatusResponse.Status[key] === responseObject.status);

    if (responseObject.masternode) {
      responseObject.masternode.status = Object.keys(GetStatusResponse.Masternode.Status)
        .find((key) => (
          GetStatusResponse.Masternode.Status[key] === responseObject.masternode.status
        ));
    }

    return responseObject;
  }

  return getStatus;
}

module.exports = getStatusFactory;
