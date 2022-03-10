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
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @param {BlockExecutionContextStackRepository} blockExecutionContextStackRepository
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {Long} latestProtocolVersion
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {BaseLogger} logger
 * @param {GroveDBStore} groveDBStore
 * @param {CreditsDistributionPoolRepository} creditsDistributionPoolRepository
 * @param {CreditsDistributionPool} creditsDistributionPool
 * @param {BlockExecutionContextStackRepository} blockExecutionContextStackRepository
 * @return {infoHandler}
 */
function infoHandlerFactory(
  blockExecutionContextStack,
  blockExecutionContextStackRepository,
  blockExecutionContext,
  latestProtocolVersion,
  updateSimplifiedMasternodeList,
  logger,
  groveDBStore,
  creditsDistributionPoolRepository,
  creditsDistributionPool,
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

    const persistedBlockExecutionContextStack = await blockExecutionContextStackRepository.fetch();

    blockExecutionContextStack.setContexts(persistedBlockExecutionContextStack.getContexts());

    const latestContext = blockExecutionContextStack.getLatest();

    if (latestContext) {
      blockExecutionContext.populate(blockExecutionContextStack.getLatest());
    }

    // Initialize Credits Distribution Pool

    if (latestContext) {
      const fetchedCreditsDistributionPool = await creditsDistributionPoolRepository.fetch();
      creditsDistributionPool.populate(fetchedCreditsDistributionPool.toJSON());
    }

    // Initialize current heights

    let lastHeight = Long.fromNumber(0);
    let lastCoreChainLockedHeight = 0;

    if (latestContext) {
      const lastHeader = blockExecutionContext.getHeader();

      lastHeight = lastHeader.height;
      lastCoreChainLockedHeight = lastHeader.coreChainLockedHeight;
    }

    contextLogger = contextLogger.child({
      height: lastHeight.toString(),
    });

    // Update SML store to latest saved core chain lock to make sure
    // that verify chain lock handler has updated SML Store to verify signatures
    if (latestContext) {
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
