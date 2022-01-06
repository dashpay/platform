const {
  tendermint: {
    abci: {
      ResponseCommit,
    },
  },
} = require('@dashevo/abci/types');

/**
 * @param {CreditsDistributionPool} creditsDistributionPool
 * @param {CreditsDistributionPoolCommonStoreRepository} creditsDistributionPoolRepository
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @param {BlockExecutionContextStackRepository} blockExecutionContextStackRepository
 * @param {rotateSignedStore} rotateSignedStore
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {AwilixContainer} container
 * @param {BaseLogger} logger
 * @param {getLatestFeatureFlag} getLatestFeatureFlag
 * @param {LRUCache} dataContractCache
 * @param {GroveDBStore} groveDBStore
 *
 * @return {commitHandler}
 */
function commitHandlerFactory(
  creditsDistributionPool,
  creditsDistributionPoolRepository,
  blockExecutionContext,
  blockExecutionContextStack,
  blockExecutionContextStackRepository,
  rotateSignedStore,
  transactionalDpp,
  container,
  logger,
  getLatestFeatureFlag,
  dataContractCache,
  groveDBStore,
) {
  /**
   * Commit ABCI Handler
   *
   * @typedef commitHandler
   *
   * @return {Promise<abci.ResponseCommit>}
   */
  async function commitHandler() {
    const { height: blockHeight } = blockExecutionContext.getHeader();

    const consensusLogger = logger.child({
      height: blockHeight.toString(),
      abciMethod: 'commit',
    });

    blockExecutionContext.setConsensusLogger(consensusLogger);

    consensusLogger.debug('Commit ABCI method requested');

    const dbTransaction = blockExecutionContext.getDBTransaction();

    try {
      // Store ST fees from the block to distribution pool
      creditsDistributionPool.incrementAmount(
        blockExecutionContext.getCumulativeFees(),
      );

      await creditsDistributionPoolRepository.store(
        creditsDistributionPool,
        blockExecutionContext.getDBTransaction(),
      );

      // Store block execution context
      blockExecutionContextStack.add(blockExecutionContext);
      blockExecutionContextStackRepository.store(
        blockExecutionContextStack,
        dbTransaction,
      );

      // Commit the current block db transactions
      await dbTransaction.commit();
    } catch (e) {
      // Abort DB transactions. It doesn't work since some of transaction may already be committed
      // and will produce "transaction in not started" error.
      if (dbTransaction.isStarted()) {
        await dbTransaction.abort();
      }

      throw e;
    }

    // Update data contract cache with new version of
    // commited data contract
    for (const dataContract of blockExecutionContext.getDataContracts()) {
      const idString = dataContract.getId().toString();

      if (dataContractCache.has(idString)) {
        dataContractCache.set(idString, dataContract);
      }
    }

    // Rotate signed store
    // Create a new GroveDB checkpoint and remove the old one
    await rotateSignedStore(blockHeight);

    const appHash = groveDBStore.getRootHash();

    consensusLogger.info(
      {
        appHash: appHash.toString('hex').toUpperCase(),
      },
      `Block commit #${blockHeight} with appHash ${appHash.toString('hex').toUpperCase()}`,
    );

    return new ResponseCommit({
      data: appHash,
    });
  }

  return commitHandler;
}

module.exports = commitHandlerFactory;
