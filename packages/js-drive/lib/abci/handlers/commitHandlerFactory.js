const {
  tendermint: {
    abci: {
      ResponseCommit,
    },
  },
} = require('@dashevo/abci/types');

/**
 * @param {CreditsDistributionPool} creditsDistributionPool
 * @param {CreditsDistributionPoolRepository} creditsDistributionPoolRepository
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @param {BlockExecutionContextStackRepository} blockExecutionContextStackRepository
 * @param {rotateSignedStore} rotateSignedStore
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {AwilixContainer} container
 * @param {BaseLogger} logger
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

    // Store ST fees from the block to distribution pool
    creditsDistributionPool.incrementAmount(
      blockExecutionContext.getCumulativeFees(),
    );

    await creditsDistributionPoolRepository.store(
      creditsDistributionPool,
      true,
    );

    // Store block execution context
    blockExecutionContextStack.add(blockExecutionContext);
    blockExecutionContextStackRepository.store(
      blockExecutionContextStack,
      true,
    );

    // Commit the current block db transactions
    await groveDBStore.commitTransaction();

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
    // TODO: We do not rotate signed state for now
    // await rotateSignedStore(blockHeight);

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
