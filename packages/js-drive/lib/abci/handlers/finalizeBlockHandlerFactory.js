const {
  tendermint: {
    abci: {
      ResponseFinalizeBlock,
    },
  },
} = require('@dashevo/abci/types');
const ReadOperation = require('@dashevo/dpp/lib/stateTransition/fee/operations/ReadOperation');
const BlockExecutionContext = require('../../blockExecution/BlockExecutionContext');
const DataContractCacheItem = require('../../dataContract/DataContractCacheItem');

/**
 *
 * @return {finalizeBlockHandler}
 * @param {GroveDBStore} groveDBStore
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContextRepository} blockExecutionContextRepository
 * @param {ProposalBlockExecutionContextCollection} proposalBlockExecutionContextCollection
 * @param {LRUCache} dataContractCache
 * @param {CoreRpcClient} coreRpcClient
 * @param {BaseLogger} logger
 * @param {ExecutionTimer} executionTimer
 */
function finalizeBlockHandlerFactory(
  groveDBStore,
  blockExecutionContext,
  blockExecutionContextRepository,
  proposalBlockExecutionContextCollection,
  dataContractCache,
  coreRpcClient,
  logger,
  executionTimer,
) {
  /**
   * @typedef finalizeBlockHandler
   *
   * @param {abci.RequestFinalizeBlock} request
   * @return {Promise<abci.ResponseFinalizeBlock>}
   */
  async function finalizeBlockHandler(request) {
    const {
      decidedLastCommit: lastCommitInfo,
      height,
      time,
      coreChainLockedHeight,
      round,
    } = request;

    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'finalizeBlock',
    });

    consensusLogger.debug('FinalizeBlock ABCI method requested');
    consensusLogger.trace({ abciRequest: request });

    const proposalBlockExecutionContext = proposalBlockExecutionContextCollection.get(round);

    proposalBlockExecutionContext.setTime(time);
    proposalBlockExecutionContext.setHeight(height);
    proposalBlockExecutionContext.setCoreChainLockedHeight(coreChainLockedHeight);

    consensusLogger.debug('Commit ABCI method requested');

    // Store block execution context
    const clonedBlockExecutionContext = new BlockExecutionContext();
    clonedBlockExecutionContext.populate(proposalBlockExecutionContext);

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
    for (const dataContract of proposalBlockExecutionContext.getDataContracts()) {
      const operations = [new ReadOperation(dataContract.toBuffer().length)];

      const cacheItem = new DataContractCacheItem(dataContract, operations);

      if (dataContractCache.has(cacheItem.getKey())) {
        dataContractCache.set(cacheItem.getKey(), cacheItem);
      }
    }

    // Send withdrawal transactions to Core
    const unsignedWithdrawalTransactionsMap = proposalBlockExecutionContext
      .getWithdrawalTransactionsMap();

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

    proposalBlockExecutionContextCollection.clear();

    const blockExecutionTimings = executionTimer.stopTimer('blockExecution');

    consensusLogger.trace(
      {
        timings: blockExecutionTimings,
      },
      `Block #${height} execution took ${blockExecutionTimings} seconds`,
    );

    return new ResponseFinalizeBlock();
  }

  return finalizeBlockHandler;
}

module.exports = finalizeBlockHandlerFactory;
