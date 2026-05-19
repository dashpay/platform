import dapiGrpc from '@dashevo/dapi-grpc';

const {
  v0: {
    GetBlockRequest,
    CorePromiseClient,
  },
} = dapiGrpc;

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getBlockByHeight}
 */
function getBlockByHeightFactory(grpcTransport) {
  /**
   * Get block by height
   * @typedef {getBlockByHeight}
   * @param {number} height
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<null|Uint8Array>}
   */
  async function getBlockByHeight(height, options = {}) {
    const getBlockRequest = new GetBlockRequest();
    getBlockRequest.setHeight(height);

    const response = await grpcTransport.request(
      CorePromiseClient,
      'getBlock',
      getBlockRequest,
      options,
    );

    const blockBinaryArray = response.getBlock();

    return new Uint8Array(blockBinaryArray);
  }

  return getBlockByHeight;
}

export default getBlockByHeightFactory;
