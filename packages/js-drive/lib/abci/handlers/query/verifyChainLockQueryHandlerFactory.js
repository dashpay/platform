const {
  tendermint: {
    abci: {
      ResponseQuery,
    },
  },
} = require('@dashevo/abci/types');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

/**
 *
 * @param {SimplifiedMasternodeList} simplifiedMasternodeList
 * @param {decodeChainLock} decodeChainLock
 * @param {detectStandaloneRegtestMode} detectStandaloneRegtestMode
 * @param {BaseLogger} logger
 * @return {verifyChainLockQueryHandler}
 */
function verifyChainLockQueryHandlerFactory(
  simplifiedMasternodeList,
  decodeChainLock,
  detectStandaloneRegtestMode,
  logger,
) {
  /**
   * @typedef verifyChainLockQueryHandler
   * @param {Object} params
   * @param {Buffer} data
   * @return {Promise<ResponseQuery>}
   */
  async function verifyChainLockQueryHandler(params, data) {
    const smlStore = simplifiedMasternodeList.getStore();

    if (smlStore === undefined) {
      throw new Error('SML Store is not defined for verify chain lock handler');
    }

    const isStandaloneRegtestMode = await detectStandaloneRegtestMode();

    const chainLock = decodeChainLock(data);

    if (isStandaloneRegtestMode) {
      logger.debug(`Fake chainLock is valid for height ${chainLock.height} against SML on height ${smlStore.tipHeight}`);

      return new ResponseQuery();
    }

    if (!chainLock.verify(smlStore)) {
      logger.debug(`Invalid chainLock for height ${chainLock.height} against SML on height ${smlStore.tipHeight}`);

      throw new InvalidArgumentAbciError(
        'Signature invalid for chainLock', chainLock.toJSON(),
      );
    }

    logger.debug(`ChainLock is valid for height ${chainLock.height} against SML on height ${smlStore.tipHeight}`);

    return new ResponseQuery();
  }

  return verifyChainLockQueryHandler;
}

module.exports = verifyChainLockQueryHandlerFactory;
