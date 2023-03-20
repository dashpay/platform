const {
  tendermint: {
    abci: {
      ResponseFinalizeBlock,
      RequestProcessProposal,
    },
  },
} = require('@dashevo/abci/types');

const lodashCloneDeep = require('lodash/cloneDeep');

/**
 *
 * @return {finalizeBlockHandler}
 * @param {GroveDBStore} groveDBStore
 * @param {BlockExecutionContextRepository} blockExecutionContextRepository
 * @param {BaseLogger} logger
 * @param {ExecutionTimer} executionTimer
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 * @param {processProposal} processProposal
 * @param {broadcastWithdrawalTransactions} broadcastWithdrawalTransactions
 * @param {createContextLogger} createContextLogger
 *
 */
function finalizeBlockHandlerFactory(
  groveDBStore,
  blockExecutionContextRepository,
  logger,
  executionTimer,
  latestBlockExecutionContext,
  proposalBlockExecutionContext,
  processProposal,
  broadcastWithdrawalTransactions,
  createContextLogger,
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

    const contextLogger = createContextLogger(logger, {
      height: height.toString(),
      round,
      abciMethod: 'finalizeBlock',
    });

    const requestToLog = lodashCloneDeep(request);
    delete requestToLog.block.data;

    contextLogger.debug('FinalizeBlock ABCI method requested');
    contextLogger.trace({ abciRequest: requestToLog });

    const lastProcessedRound = proposalBlockExecutionContext.getRound();

    if (lastProcessedRound !== round) {
      contextLogger.warn({
        lastProcessedRound,
        round,
      }, `Finalizing previously executed round ${round} instead of the last known ${lastProcessedRound}`);

      const {
        block: {
          header: {
            time,
            version,
            proposerProTxHash,
            proposedAppVersion,
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
        proposedAppVersion,
        round,
      });

      await processProposal(processProposalRequest, contextLogger);
    }

    // Send withdrawal transactions to Core
    const unsignedWithdrawalTransactionsMap = proposalBlockExecutionContext
      .getWithdrawalTransactionsMap();

    const { thresholdVoteExtensions } = commitInfo;

    await broadcastWithdrawalTransactions(
      proposalBlockExecutionContext,
      thresholdVoteExtensions,
      unsignedWithdrawalTransactionsMap,
    );

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

    proposalBlockExecutionContext.reset();

    const blockExecutionTimings = executionTimer.stopTimer('blockExecution');

    contextLogger.info(
      {
        timings: blockExecutionTimings,
      },
      `Block #${height} finalized in ${round + 1} round(s) and ${blockExecutionTimings} seconds`,
    );

    return new ResponseFinalizeBlock();
  }

  return finalizeBlockHandler;
}

module.exports = finalizeBlockHandlerFactory;
