const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const featureFlagTypes = require('@dashevo/feature-flags-contract/lib/featureFlagTypes');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

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

      throw new InvalidArgumentAbciError(
        'Invalid ChainLock format', { chainLock: data.toString('hex') },
      );
    }

    let verifyLLMQSignaturesWithCoreFeatureFlag;

    const header = blockExecutionContext.getHeader();
    if (header) {
      verifyLLMQSignaturesWithCoreFeatureFlag = await getLatestFeatureFlag(
        featureFlagTypes.VERIFY_LLMQ_SIGS_WITH_CORE,
        header.height,
      );
    }

    if (!verifyLLMQSignaturesWithCoreFeatureFlag || !verifyLLMQSignaturesWithCoreFeatureFlag.get('enabled')) {
      const smlStore = simplifiedMasternodeList.getStore();

      if (smlStore === undefined) {
        throw new Error('SML Store is not defined for verify chain lock handler');
      }

      // Here dashcore lib is used to verify chain lock,
      // but this approach doesn’t handle chain locks created by old quorums
      // that’a why a Core RPC method is used otherwise
      if (!chainLock.verify(smlStore)) {
        logger.debug(`Invalid chainLock for height ${chainLock.height} against SML on height ${smlStore.tipHeight}`);

        throw new InvalidArgumentAbciError(
          'ChainLock verification failed', chainLock.toJSON(),
        );
      }

      logger.debug(`ChainLock is valid for height ${chainLock.height} against SML on height ${smlStore.tipHeight}`);

      return new ResponseQuery();
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

      throw new InvalidArgumentAbciError(
        'ChainLock verification failed', chainLock.toJSON(),
      );
    }

    logger.debug(`ChainLock is valid for height ${chainLock.height}`);

    return new ResponseQuery();
  }

  return verifyChainLockQueryHandler;
}

module.exports = verifyChainLockQueryHandlerFactory;
