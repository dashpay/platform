const {
  abci: {
    ResponseCommit,
  },
} = require('abci/types');

/**
 * @param {BlockchainState} blockchainState
 * @param {BlockchainStateLevelDBRepository} blockchainStateRepository
 * @param {BlockExecutionDBTransactions} blockExecutionDBTransactions
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {DocumentDatabaseManager} documentDatabaseManager
 * @param {BaseLogger} logger
 *
 * @return {commitHandler}
 */
function commitHandlerFactory(
  blockchainState,
  blockchainStateRepository,
  blockExecutionDBTransactions,
  blockExecutionContext,
  documentDatabaseManager,
  logger,
) {
  /**
   * Commit ABCI handler
   *
   * @typedef commitHandler
   *
   * @return {Promise<abci.ResponseCommit>}
   */
  async function commitHandler() {
    const { height: blockHeight } = blockExecutionContext.getHeader();

    logger.info(`Block commit #${blockHeight}`);

    // We don't build state tree for now
    // so appHash always empty
    const appHash = Buffer.alloc(0);

    try {
      // Create document databases for dataContracts created in the current block
      for (const dataContract of blockExecutionContext.getDataContracts()) {
        await documentDatabaseManager.create(dataContract);
      }

      // Commit DB transactions
      await blockExecutionDBTransactions.commit();

      // Update blockchain state
      blockchainState.setLastBlockAppHash(appHash);

      // Store ST fees from the block to distribution pool
      blockchainState.setCreditsDistributionPool(blockExecutionContext.getAccumulativeFees());

      await blockchainStateRepository.store(blockchainState);
    } catch (e) {
      // Abort DB transactions
      await blockExecutionDBTransactions.abort();

      for (const dataContract of blockExecutionContext.getDataContracts()) {
        await documentDatabaseManager.drop(dataContract);
      }

      throw e;
    } finally {
      // Reset block execution state
      blockExecutionContext.reset();
    }

    return new ResponseCommit({
      data: appHash,
    });
  }

  return commitHandler;
}

module.exports = commitHandlerFactory;
