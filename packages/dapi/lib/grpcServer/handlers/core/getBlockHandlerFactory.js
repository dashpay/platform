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
 * @param {InsightAPI} insightAPI
 * @returns {getBlockHandler}
 */
function getBlockHandlerFactory(insightAPI) {
  /**
   * @typedef getBlockHandler
   * @param {Object} call
   * @return {Promise<GetBlockResponse>}
   */
  async function getBlockHandler(call) {
    const { request } = call;

    const height = request.getHeight();
    const hash = request.getHash();

    if (!hash && !height) {
      throw new InvalidArgumentGrpcError('hash or height is not specified');
    }

    let serializedBlock;

    if (hash) {
      try {
        serializedBlock = await insightAPI.getRawBlockByHash(hash);
      } catch (e) {
        if (e.statusCode === 404) {
          throw new NotFoundGrpcError('Block not found');
        }

        throw e;
      }
    } else {
      try {
        serializedBlock = await insightAPI.getRawBlockByHeight(height);
      } catch (e) {
        if (e.statusCode === 400) {
          throw new InvalidArgumentGrpcError('Invalid block height');
        }

        throw e;
      }
    }

    const response = new GetBlockResponse();
    const serializedBlockBuffer = Buffer.from(serializedBlock, 'hex');
    response.setBlock(serializedBlockBuffer);

    return response;
  }

  return getBlockHandler;
}

module.exports = getBlockHandlerFactory;
