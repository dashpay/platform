const {
  v0: {
    GetBlockResponse,
  },
} = require('@dashevo/dapi-grpc');

const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      NotFoundGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

/**
 * @param {CoreRpcClient} coreRPCClient
 * @returns {getBlockHandler}
 */
function getBlockHandlerFactory(coreRPCClient) {
  /**
   * @typedef getBlockHandler
   * @param {Object} call
   * @return {Promise<GetBlockResponse>}
   */
  async function getBlockHandler(call) {
    const { request } = call;

    const height = request.getHeight();
    let hash = request.getHash();

    if (!hash && !height) {
      throw new InvalidArgumentGrpcError('hash or height is not specified');
    }

    let serializedBlock;

    if (!hash) {
      try {
        hash = await coreRPCClient.getBlockHash(height);
      } catch (e) {
        if (e.code === -8) {
          // Block height out of range
          throw new NotFoundGrpcError('Invalid block height');
        }
        if (e.code === -1) {
          // Invalid argument (not integer or integer out of range)
          throw new InvalidArgumentGrpcError(e.message);
        }

        throw e;
      }
    }

    try {
      serializedBlock = await coreRPCClient.getRawBlock(hash);
    } catch (e) {
      if (e.code === -5) {
        throw new NotFoundGrpcError('Block not found');
      }

      throw e;
    }

    const response = new GetBlockResponse();
    const serializedBlockBuffer = Buffer.from(serializedBlock, 'hex');
    response.setBlock(serializedBlockBuffer);

    return response;
  }

  return getBlockHandler;
}

module.exports = getBlockHandlerFactory;
