const {
  tendermint: {
    abci: {
      ResponseFinalizeBlock,
      RequestProcessProposal,
    },
  },
} = require('@dashevo/abci/types');

/**
 *
 * @return {finalizeBlockHandler}
 * @param {GroveDBStore} groveDBStore
 * @param {BlockExecutionContextRepository} blockExecutionContextRepository
 * @param {CoreRpcClient} coreRpcClient
 * @param {BaseLogger} logger
 * @param {ExecutionTimer} executionTimer
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 * @param {processProposal} processProposal
 */
function finalizeBlockHandlerFactory(
  groveDBStore,
  blockExecutionContextRepository,
  coreRpcClient,
  logger,
  executionTimer,
  latestBlockExecutionContext,
  proposalBlockExecutionContext,
  processProposal,
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
      round,
      abciMethod: 'finalizeBlock',
    });

    consensusLogger.debug('FinalizeBlock ABCI method requested');
    consensusLogger.trace({ abciRequest: request });

    const lastProcessedRound = proposalBlockExecutionContext.getRound();

    if (lastProcessedRound !== round) {
      consensusLogger.warn({
        lastProcessedRound,
        round,
      }, `Finalizing previously executed round ${round} instead of the last known ${lastProcessedRound}`);

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

      const processProposalRequest = new RequestProcessProposal({
        height,
        txs,
        coreChainLockedHeight,
        version,
        proposedLastCommit: commitInfo,
        time,
        proposerProTxHash,
        round,
      });

      await processProposal(processProposalRequest, consensusLogger);

      // Revert consensus logger
      proposalBlockExecutionContext.setConsensusLogger(consensusLogger);
    }

    proposalBlockExecutionContext.setLastCommitInfo(commitInfo);

    // Store proposal block execution context
    await blockExecutionContextRepository.store(
      proposalBlockExecutionContext,
      {
        useTransaction: true,
      },
    );

    // Commit the current block db transactions into storage
    await groveDBStore.commitTransaction();

    // Update last block execution context with proposal data
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

    consensusLogger.info(
      {
        timings: blockExecutionTimings,
      },
      `Block #${height} finalized in ${round + 1} rounds and ${blockExecutionTimings} seconds`,
    );

    return new ResponseFinalizeBlock();
  }

  return finalizeBlockHandler;
}

module.exports = finalizeBlockHandlerFactory;
