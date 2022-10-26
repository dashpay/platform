const {
  tendermint: {
    abci: {
      ResponsePrepareProposal,
    },
  },
} = require('@dashevo/abci/types');

const txAction = {
  UNKNOWN: 0, // Unknown action
  UNMODIFIED: 1, // The Application did not modify this transaction.
  ADDED: 2, // The Application added this transaction.
  REMOVED: 3, // The Application wants this transaction removed from the proposal and the mempool.
};

/**
 * @param {deliverTx} deliverTx
 * @param {BaseLogger} logger
 * @param {ProposalBlockExecutionContextCollection} proposalBlockExecutionContextCollection
 * @param {beginBlock} beginBlock
 * @param {endBlock} endBlock
 * @param {updateCoreChainLock} updateCoreChainLock
 * @param {ExecutionTimer} executionTimer
 * @return {prepareProposalHandler}
 */
function prepareProposalHandlerFactory(
  deliverTx,
  logger,
  proposalBlockExecutionContextCollection,
  beginBlock,
  endBlock,
  updateCoreChainLock,
  executionTimer,
) {
  /**
   * @typedef prepareProposalHandler
   * @param {abci.RequestPrepareProposal} request
   * @return {Promise<abci.ResponsePrepareProposal>}
   */
  async function prepareProposalHandler(request) {
    const {
      height,
      maxTxBytes,
      txs,
      coreChainLockedHeight,
      version,
      localLastCommit: lastCommitInfo,
      time,
      proposerProTxHash,
      round,
    } = request;
    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'prepareProposal',
    });

    consensusLogger.info(
      {
        height,
      },
      `Prepare proposal #${height}`,
    );
    consensusLogger.debug('PrepareProposal ABCI method requested');
    consensusLogger.trace({ abciRequest: request });

    executionTimer.clearTimer('roundExecution');
    executionTimer.startTimer('roundExecution');

    executionTimer.clearTimer('blockExecution');
    executionTimer.startTimer('blockExecution');

    await beginBlock(
      {
        lastCommitInfo,
        height,
        coreChainLockedHeight,
        version,
        time,
        proposerProTxHash: Buffer.from(proposerProTxHash),
      },
      consensusLogger,
    );

    const proposalBlockExecutionContext = proposalBlockExecutionContextCollection.get(round);

    let totalSizeBytes = 0;

    const txRecords = [];
    const txResults = [];
    let validTxCount = 0;
    let invalidTxCount = 0;
    let storageFee = 0;
    let processingFee = 0;

    for (const tx of txs) {
      totalSizeBytes += tx.length;

      if (totalSizeBytes > maxTxBytes) {
        break;
      }

      txRecords.push({
        tx,
        action: txAction.UNMODIFIED,
      });

      const {
        txResult,
        actualProcessingFee,
        actualStorageFee,
      } = await deliverTx(tx, round, consensusLogger);

      if (txResult.code === 0) {
        validTxCount += 1;
        storageFee += actualStorageFee;
        processingFee += actualProcessingFee;
      } else {
        invalidTxCount += 1;
      }

      txResults.push(txResult);
    }

    proposalBlockExecutionContext.setConsensusLogger(consensusLogger);

    const coreChainLockUpdate = await updateCoreChainLock(round, consensusLogger);

    const {
      consensusParamUpdates,
      validatorSetUpdate,
      appHash,
    } = await endBlock(height, round, processingFee, storageFee, consensusLogger);

    const prepareProposalTimings = executionTimer.stopTimer('roundExecution');

    consensusLogger.info(
      {
        validTxCount,
        invalidTxCount,
      },
      `Prepare proposal #${height} with appHash ${appHash.toString('hex').toUpperCase()}`
      + ` (valid txs = ${validTxCount}, invalid txs = ${invalidTxCount}). Took ${prepareProposalTimings} seconds`,
    );

    return new ResponsePrepareProposal({
      appHash,
      txResults,
      consensusParamUpdates,
      validatorSetUpdate,
      coreChainLockUpdate,
      txRecords,
    });
  }

  return prepareProposalHandler;
}

module.exports = prepareProposalHandlerFactory;
