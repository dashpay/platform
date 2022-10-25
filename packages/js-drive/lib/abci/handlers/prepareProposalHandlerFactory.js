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
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {beginBlock} beginBlock
 * @param {endBlock} endBlock
 * @param {updateCoreChainLock} updateCoreChainLock
 * @param {ExecutionTimer} executionTimer
 * @return {prepareProposalHandler}
 */
function prepareProposalHandlerFactory(
  deliverTx,
  logger,
  blockExecutionContext,
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

    executionTimer.clearTimer('prepareProposal');
    executionTimer.startTimer('prepareProposal');

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

    let totalSizeBytes = 0;

    const txRecords = [];
    const txResults = [];
    let validTxCount = 0;
    let invalidTxCount = 0;

    for (const tx of txs) {
      totalSizeBytes += tx.length;

      if (totalSizeBytes > maxTxBytes) {
        break;
      }

      txRecords.push({
        tx,
        action: txAction.UNMODIFIED,
      });

      const txResult = await deliverTx(tx, consensusLogger);

      if (txResult.code === 0) {
        validTxCount += 1;
      } else {
        invalidTxCount += 1;
      }

      txResults.push(txResult);
    }

    blockExecutionContext.setConsensusLogger(consensusLogger);

    const processingFees = blockExecutionContext.getCumulativeProcessingFee();
    const storageFees = blockExecutionContext.getCumulativeStorageFee();

    const coreChainLockUpdate = await updateCoreChainLock(consensusLogger);

    const {
      consensusParamUpdates,
      validatorSetUpdate,
      appHash,
    } = await endBlock(height, processingFees, storageFees, consensusLogger);

    const prepareProposalTimings = executionTimer.stopTimer('prepareProposal');

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
