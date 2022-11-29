const {
  tendermint: {
    abci: {
      ResponseFinalizeBlock,
    },
  },
} = require('@dashevo/abci/types');

/**
 *
 * @return {finalizeBlockHandler}
 * @param {GroveDBStore} groveDBStore
 * @param {BlockExecutionContextRepository} blockExecutionContextRepository
 * @param {LRUCache} dataContractCache
 * @param {CoreRpcClient} coreRpcClient
 * @param {BaseLogger} logger
 * @param {ExecutionTimer} executionTimer
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 * @param {processProposalHandler} processProposalHandler
 */
function finalizeBlockHandlerFactory(
  groveDBStore,
  blockExecutionContextRepository,
  coreRpcClient,
  logger,
  executionTimer,
  latestBlockExecutionContext,
  proposalBlockExecutionContext,
  processProposalHandler,
) {
  /**
   * @typedef finalizeBlockHandler
   *
   * @param {abci.RequestFinalizeBlock} request
   * @return {Promise<abci.ResponseFinalizeBlock>}
   */
  async function finalizeBlockHandler(request) {
    const {
      commit: commitInfo,
      height,
      round,
    } = request;

    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'finalizeBlock',
    });

    consensusLogger.debug('FinalizeBlock ABCI method requested');
    consensusLogger.trace({ abciRequest: request });

    if (
      proposalBlockExecutionContext.getHeight().toNumber() !== height.toNumber()
      || proposalBlockExecutionContext.getRound() !== round
    ) {
      consensusLogger.warn('Height or round in execution context do not equal request values.');

      const {
        block: {
          header: {
            time,
            version,
            proposerProTxHash,
            coreChainLockedHeight,
          },
          data: {
            txs,
          },
        },
      } = request;

      await processProposalHandler({
        height,
        txs,
        coreChainLockedHeight,
        version,
        proposedLastCommit: commitInfo,
        time,
        proposerProTxHash,
        round,
      });
    }

    proposalBlockExecutionContext.setLastCommitInfo(commitInfo);

    // Store block execution context
    await blockExecutionContextRepository.store(
      proposalBlockExecutionContext,
      {
        useTransaction: true,
      },
    );

    // Commit the current block db transactions
    await groveDBStore.commitTransaction();

    latestBlockExecutionContext.populate(proposalBlockExecutionContext);

    // Send withdrawal transactions to Core
    const unsignedWithdrawalTransactionsMap = proposalBlockExecutionContext
      .getWithdrawalTransactionsMap();

    const { thresholdVoteExtensions } = commitInfo;

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

    proposalBlockExecutionContext.reset();

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
