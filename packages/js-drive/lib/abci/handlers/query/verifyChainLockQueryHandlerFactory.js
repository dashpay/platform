const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const InvalidArgumentGrpcError = require('@dashevo/grpc-common/lib/server/error/InvalidArgumentGrpcError');

/**
 *
 * @param {SimplifiedMasternodeList} simplifiedMasternodeList
 * @param {decodeChainLock} decodeChainLock
 * @param {getLatestFeatureFlag} getLatestFeatureFlag
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {RpcClient} coreRpcClient
 * @param {BaseLogger} logger
 * @return {verifyChainLockQueryHandler}
 */
function verifyChainLockQueryHandlerFactory(
  simplifiedMasternodeList,
  decodeChainLock,
  getLatestFeatureFlag,
  blockExecutionContext,
  coreRpcClient,
  logger,
) {
  /**
   * @typedef verifyChainLockQueryHandler
   * @param {Object} params
   * @param {Buffer} data
   * @return {Promise<ResponseQuery>}
   */
  async function verifyChainLockQueryHandler(params, data) {
    let chainLock;
    try {
      chainLock = decodeChainLock(data);
    } catch (e) {
      logger.debug(
        {
          chainLock: data.toString('hex'),
        },
        'Invalid chainLock format',
      );

      throw new InvalidArgumentGrpcError(
        'Invalid ChainLock format', { chainLock: data.toString('hex') },
      );
    }

    let isVerified;
    try {
      ({ result: isVerified } = await coreRpcClient.verifyChainLock(
        chainLock.blockHash.toString('hex'),
        chainLock.signature.toString('hex'),
        chainLock.height,
      ));
    } catch (e) {
      // Invalid signature format
      // Parse error
      if ([-8, -32700].includes(e.code)) {
        return false;
      }

      throw e;
    }

    if (!isVerified) {
      logger.debug(`Invalid chainLock for height ${chainLock.height}`);

      throw new InvalidArgumentGrpcError(
        'ChainLock verification failed', chainLock.toJSON(),
      );
    }

    logger.debug(`ChainLock is valid for height ${chainLock.height}`);

    return new ResponseQuery();
  }

  return verifyChainLockQueryHandler;
}

module.exports = verifyChainLockQueryHandlerFactory;
