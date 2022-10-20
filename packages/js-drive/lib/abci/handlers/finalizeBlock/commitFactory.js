const ReadOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/ReadOperation');
const DataContractCacheItem = require('../../../dataContract/DataContractCacheItem');
const BlockExecutionContext = require('../../../blockExecution/BlockExecutionContext');

/**
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContextRepository} blockExecutionContextRepository
 * @param {GroveDBStore} groveDBStore
 * @param {LRUCache} dataContractCache
 * @param {CoreRpcClient} coreRpcClient
 *
 * @return {commit}
 */
function commitFactory(
  blockExecutionContext,
  blockExecutionContextRepository,
  dataContractCache,
  groveDBStore,
  coreRpcClient,
) {
  /**
   * @typedef commit
   *
   * @param {ILastCommitInfo} lastCommitInfo
   * @param {BaseLogger} logger
   *
   * @return {Promise<{ appHash: Buffer }>}
   */
  async function commit(lastCommitInfo, logger) {
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

    blockExecutionContextRepository.store(
      clonedBlockExecutionContext,
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

    // Send withdrawal transactions to Core
    const unsignedWithdrawalTransactionsMap = blockExecutionContext.getWithdrawalTransactionsMap();

    const { thresholdVoteExtensions } = lastCommitInfo;

    for (const { extension, signature } of (thresholdVoteExtensions || [])) {
      const withdrawalTransactionHash = extension.toString('hex');

      const unsignedWithdrawalTransactionBytes = unsignedWithdrawalTransactionsMap[
        withdrawalTransactionHash
      ];

      if (unsignedWithdrawalTransactionBytes) {
        const transactionBytes = Buffer.concat([
          unsignedWithdrawalTransactionBytes,
          signature,
        ]);

        // TODO: think about Core error handling
        await coreRpcClient.sendRawTransaction(transactionBytes.toString('hex'));
      }
    }
  }

  return commit;
}

module.exports = commitFactory;
