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
  createContextLogger,
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
    let contextLogger = createContextLogger(logger, {
      abciMethod: 'info',
    });

    contextLogger.debug('Info ABCI method requested');
    contextLogger.trace({ abciRequest: request });

    // Initialize current heights

    let lastHeight = Long.fromNumber(0);
    let lastCoreChainLockedHeight = 0;

    // Initialize latest Block Execution Context
    const persistedBlockExecutionContext = await blockExecutionContextRepository.fetch();
    if (!persistedBlockExecutionContext.isEmpty()) {
      latestBlockExecutionContext.populate(persistedBlockExecutionContext);

      lastHeight = latestBlockExecutionContext.getHeight();
      lastCoreChainLockedHeight = latestBlockExecutionContext.getCoreChainLockedHeight();

      contextLogger = createContextLogger(contextLogger, {
        height: lastHeight.toString(),
      });

      // Update SML store to latest saved core chain lock to make sure
      // that verify chain lock handler has updated SML Store to verify signatures
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
