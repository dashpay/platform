const ReadOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/ReadOperation');
const DataContractCacheItem = require('../../../dataContract/DataContractCacheItem');
const BlockExecutionContext = require('../../../blockExecution/BlockExecutionContext');

/**
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @param {BlockExecutionContextStackRepository} blockExecutionContextStackRepository
 * @param {rotateSignedStore} rotateSignedStore
 * @param {GroveDBStore} groveDBStore
 * @param {LRUCache} dataContractCache
 *
 * @return {commit}
 */
function commitFactory(
  blockExecutionContext,
  blockExecutionContextStack,
  blockExecutionContextStackRepository,
  rotateSignedStore,
  dataContractCache,
  groveDBStore,
) {
  /**
   * @typedef commit
   *
   * @param {BaseLogger} logger
   * @return {Promise<{ appHash: Buffer }>}
   */
  async function commit(logger) {
    const blockHeight = blockExecutionContext.getHeight();

    const consensusLogger = logger.child({
      height: blockHeight.toString(),
      abciMethod: 'finalizeBlock#commit',
    });

    blockExecutionContext.setConsensusLogger(consensusLogger);

    consensusLogger.debug('Commit ABCI method requested');

    // Store block execution context
    const clonedBlockExecutionContext = new BlockExecutionContext();
    clonedBlockExecutionContext.populate(blockExecutionContext);

    blockExecutionContextStack.add(clonedBlockExecutionContext);

    blockExecutionContextStackRepository.store(
      blockExecutionContextStack,
      {
        useTransaction: true,
      },
    );

    // Commit the current block db transactions
    await groveDBStore.commitTransaction();

    // Update data contract cache with new version of
    // committed data contract
    for (const dataContract of blockExecutionContext.getDataContracts()) {
      const operations = [new ReadOperation(dataContract.toBuffer().length)];

      const cacheItem = new DataContractCacheItem(dataContract, operations);

      if (dataContractCache.has(cacheItem.getKey())) {
        dataContractCache.set(cacheItem.getKey(), cacheItem);
      }
    }

    // Rotate signed store
    // Create a new GroveDB checkpoint and remove the old one
    // TODO: We do not rotate signed state for now
    // await rotateSignedStore(blockHeight);

    const appHash = await groveDBStore.getRootHash();

    consensusLogger.info(
      {
        appHash: appHash.toString('hex').toUpperCase(),
      },
      `Block commit #${blockHeight} with appHash ${appHash.toString('hex').toUpperCase()}`,
    );

    return {
      appHash,
    };
  }

  return commit;
}

module.exports = commitFactory;
