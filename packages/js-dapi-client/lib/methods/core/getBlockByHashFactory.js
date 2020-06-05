const {
  GetBlockRequest,
  CorePromiseClient,
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getBlockByHash}
 */
function getBlockByHashFactory(grpcTransport) {
  /**
   * Get block by hash
   *
   * @typedef {getBlockByHash}
   * @param {string} hash
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<null|Buffer>}
   */
  async function getBlockByHash(hash, options = {}) {
    const getBlockRequest = new GetBlockRequest();
    getBlockRequest.setHash(hash);

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

  return getBlockByHash;
}

module.exports = getBlockByHashFactory;
