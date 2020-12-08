const {
  abci: {
    ResponseCommit,
  },
} = require('abci/types');

/**
 * @param {ChainInfo} chainInfo
 * @param {ChainInfoExternalStoreRepository} chainInfoRepository
 * @param {CreditsDistributionPool} creditsDistributionPool
 * @param {CreditsDistributionPoolCommonStoreRepository} creditsDistributionPoolRepository
 * @param {BlockExecutionDBTransactions} blockExecutionDBTransactions
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {DocumentDatabaseManager} documentDatabaseManager
 * @param {RootTree} rootTree
 * @param {BaseLogger} logger
 *
 * @return {commitHandler}
 */
function commitHandlerFactory(
  chainInfo,
  chainInfoRepository,
  creditsDistributionPool,
  creditsDistributionPoolRepository,
  blockExecutionDBTransactions,
  blockExecutionContext,
  documentDatabaseManager,
  rootTree,
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

    try {
      // Create document databases for dataContracts created in the current block
      for (const dataContract of blockExecutionContext.getDataContracts()) {
        await documentDatabaseManager.create(dataContract);
      }

      // Commit DB transactions
      await blockExecutionDBTransactions.commit();

      // Store ST fees from the block to distribution pool
      creditsDistributionPool.setAmount(
        blockExecutionContext.getAccumulativeFees(),
      );

      await chainInfoRepository.store(chainInfo);
      await creditsDistributionPoolRepository.store(creditsDistributionPool);

      rootTree.rebuild();
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
      data: rootTree.getRootHash(),
    });
  }

  return commitHandler;
}

module.exports = commitHandlerFactory;
