const {
  tendermint: {
    abci: {
      ResponseProcessProposal,
    },
  },
} = require('@dashevo/abci/types');

const proposalStatus = {
  UNKNOWN: 0, // Unknown status. Returning this from the application is always an error.
  ACCEPT: 1, // Status that signals that the application finds the proposal valid.
  REJECT: 2, // Status that signals that the application finds the proposal invalid.
};

/**
 * @param {deliverTx} deliverTx
 * @param {BaseLogger} logger
 * @param {GroveDBStore} groveDBStore
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {beginBlock} beginBlock
 * @param {endBlock} endBlock
 * @param {ExecutionTimer} executionTimer
 * @return {processProposalHandler}
 */
function processProposalHandlerFactory(
  deliverTx,
  logger,
  groveDBStore,
  blockExecutionContext,
  beginBlock,
  endBlock,
  executionTimer,
) {
  /**
   * @typedef processProposalHandler
   * @return {Promise<abci.ResponseProcessProposal>}
   */
  async function processProposalHandler(request) {
    const {
      height,
      txs,
      coreChainLockedHeight,
      version,
      proposedLastCommit: lastCommitInfo,
      time,
      proposerProTxHash,
    } = request;

    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'processProposal',
    });

    consensusLogger.info(
      {
        height,
      },
      `Process proposal #${height}`,
    );
    consensusLogger.debug('ProcessProposal ABCI method requested');
    consensusLogger.trace({ abciRequest: request });

    executionTimer.clearTimer('processProposal');
    executionTimer.startTimer('processProposal');

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

    const txResults = [];
    let validTxCount = 0;
    let invalidTxCount = 0;

    for (const tx of txs) {
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

    const {
      consensusParamUpdates,
      validatorSetUpdate,
      appHash,
    } = await endBlock(height, processingFees, storageFees, consensusLogger);

    const processProposalTimings = executionTimer.stopTimer('processProposal');

    consensusLogger.info(
      {
        validTxCount,
        invalidTxCount,
      },
      `Process proposal #${height} with appHash ${appHash.toString('hex').toUpperCase()}`
      + ` (valid txs = ${validTxCount}, invalid txs = ${invalidTxCount}). Took ${processProposalTimings} seconds`,
    );

    return new ResponseProcessProposal({
      status: proposalStatus.ACCEPT,

      appHash,
      txResults,
      consensusParamUpdates,
      validatorSetUpdate,
    });
  }

  return processProposalHandler;
}

module.exports = processProposalHandlerFactory;
