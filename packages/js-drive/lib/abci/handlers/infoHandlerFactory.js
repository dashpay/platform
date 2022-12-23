const {
  tendermint: {
    abci: {
      ResponseInfo,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');

const { version: driveVersion } = require('../../../package.json');

/**
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @param {BlockExecutionContextRepository} blockExecutionContextRepository
 * @param {Long} latestProtocolVersion
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {BaseLogger} logger
 * @param {GroveDBStore} groveDBStore
 * @return {infoHandler}
 */
function infoHandlerFactory(
  latestBlockExecutionContext,
  blockExecutionContextRepository,
  latestProtocolVersion,
  updateSimplifiedMasternodeList,
  logger,
  groveDBStore,
) {
  /**
   * Info ABCI handler
   *
   * @typedef infoHandler
   *
   * @param {abci.RequestInfo} request
   * @return {Promise<ResponseInfo>}
   */
  async function infoHandler(request) {
    let contextLogger = logger.child({
      abciMethod: 'info',
    });

    contextLogger.debug('Info ABCI method requested');
    contextLogger.trace({ abciRequest: request });

    // Initialize Block Execution Contexts

    const persistedBlockExecutionContext = await blockExecutionContextRepository.fetch();
    if (persistedBlockExecutionContext) {
      latestBlockExecutionContext.populate(persistedBlockExecutionContext);
    }

    // Initialize current heights

    let lastHeight = Long.fromNumber(0);
    let lastCoreChainLockedHeight = 0;

    if (!latestBlockExecutionContext.isEmpty()) {
      lastHeight = latestBlockExecutionContext.getHeight();
      lastCoreChainLockedHeight = latestBlockExecutionContext.getCoreChainLockedHeight();
    }

    contextLogger = contextLogger.child({
      height: lastHeight.toString(),
    });

    // Update SML store to latest saved core chain lock to make sure
    // that verify chain lock handler has updated SML Store to verify signatures
    if (!latestBlockExecutionContext.isEmpty()) {
      await updateSimplifiedMasternodeList(lastCoreChainLockedHeight, {
        logger: contextLogger,
      });
    }

    const appHash = await groveDBStore.getRootHash();

    contextLogger.info(
      {
        lastHeight: lastHeight.toString(),
        appHash: appHash.toString('hex').toUpperCase(),
        latestProtocolVersion: latestProtocolVersion.toString(),
      },
      `Start processing from block #${lastHeight} with appHash ${appHash.toString('hex').toUpperCase()}`,
    );

    return new ResponseInfo({
      version: driveVersion,
      appVersion: latestProtocolVersion,
      lastBlockHeight: lastHeight,
      lastBlockAppHash: appHash,
    });
  }

  return infoHandler;
}

module.exports = infoHandlerFactory;
