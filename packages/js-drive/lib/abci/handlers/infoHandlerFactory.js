const {
  tendermint: {
    abci: {
      ResponseInfo,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');

const { asValue } = require('awilix');

const { version: driveVersion } = require('../../../package.json');

const NoPreviousBlockExecutionStoreTransactionsFoundError = require('./errors/NoPreviousBlockExecutionStoreTransactionsFoundError');

/**
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {Long} latestProtocolVersion
 * @param {RootTree} rootTree
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {BaseLogger} logger
 * @param {
 *   PreviousBlockExecutionStoreTransactionsRepository
 * } previousBlockExecutionStoreTransactionsRepository
 * @param {AwilixContainer} container
 * @return {infoHandler}
 */
function infoHandlerFactory(
  blockExecutionContext,
  latestProtocolVersion,
  rootTree,
  updateSimplifiedMasternodeList,
  logger,
  previousBlockExecutionStoreTransactionsRepository,
  container,
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

    // Update CreditsDistributionPool

    const lastHeader = blockExecutionContext.getHeader();

    let lastHeight = Long.fromNumber(0);
    let lastCoreChainLockedHeight = 0;
    if (lastHeader) {
      lastHeight = lastHeader.height;
      lastCoreChainLockedHeight = lastHeader.coreChainLockedHeight;
    }

    contextLogger = contextLogger.child({
      height: lastHeight.toString(),
    });

    if (lastHeader) {
      // If the current block is higher than 1 we need to obtain previous block data
      if (!container.has('previousBlockExecutionStoreTransactions')) {
        // If container doesn't have previous transactions, load them from file (node cold start)
        const previousBlockExecutionStoreTransactions = (
          await previousBlockExecutionStoreTransactionsRepository.fetch()
        );

        if (!previousBlockExecutionStoreTransactions) {
          throw new NoPreviousBlockExecutionStoreTransactionsFoundError();
        }

        container.register({
          previousBlockExecutionStoreTransactions: asValue(previousBlockExecutionStoreTransactions),
        });
      }

      // Update SML store to latest saved core chain lock to make sure
      // that verify chain lock handler has updated SML Store to verify signatures

      await updateSimplifiedMasternodeList(lastCoreChainLockedHeight, {
        logger: contextLogger,
      });
    }

    const appHash = rootTree.getRootHash();

    contextLogger.info(
      {
        lastHeight: lastHeight.toString(),
        lastCoreChainLockedHeight,
        appHash: appHash.toString('hex').toUpperCase(),
        driveVersion,
        latestProtocolVersion: latestProtocolVersion.toString(),
      },
      `Start processing from block #${lastHeight} with appHash ${appHash.toString('hex').toUpperCase() || 'nil'}`,
    );

    return new ResponseInfo({
      version: driveVersion,
      appVersion: latestProtocolVersion,
      lastBlockHeight: lastHeight,
      lastBlockAppHash: appHash,
      lastCoreChainLockedHeight,
    });
  }

  return infoHandler;
}

module.exports = infoHandlerFactory;
