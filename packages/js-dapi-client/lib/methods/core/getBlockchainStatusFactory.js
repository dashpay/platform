import dapiGrpc from '@dashevo/dapi-grpc';

const {
  v0: {
    GetBlockchainStatusRequest,
    GetBlockchainStatusResponse,
    CorePromiseClient,
  },
} = dapiGrpc;

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

    // Respond with Uint8Arrays instead of base64 for binary fields

    if (response.getChain()) {
      if (response.getChain()
        .getBestBlockHash()) {
        responseObject.chain.bestBlockHash = new Uint8Array(response.getChain()
          .getBestBlockHash());
      }

      if (response.getChain()
        .getChainWork()) {
        responseObject.chain.chainWork = new Uint8Array(response.getChain()
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

export default getBlockchainStatusFactory;
