const {
  v0: {
    GetBlockRequest,
    CorePromiseClient,
  },
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');


/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getBlockByHeight}
 */
function getBlockByHeightFactory(grpcTransport) {
  /**
   * Get block by height
   *
   * @typedef {getBlockByHeight}
   * @param {number} height
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<null|Buffer>}
   */
  async function getBlockByHeight(height, options = {}) {
    const getBlockRequest = new GetBlockRequest();
    getBlockRequest.setHeight(height);

    let response;
    try {
      response = await grpcTransport.request(
        CorePromiseClient,
        'getBlock',
        getBlockRequest,
        options,
      );
    } catch (e) {
      if (e.code === grpcErrorCodes.NOT_FOUND) {
        return null;
      }

      throw e;
    }

    const blockBinaryArray = response.getBlock();

    return Buffer.from(blockBinaryArray);
  }

  return getBlockByHeight;
}

module.exports = getBlockByHeightFactory;
