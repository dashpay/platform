const {
  tendermint: {
    abci: {
      ResponseProcessProposal,
    },
    types: {
      CoreChainLock,
    },
  },
} = require('@dashevo/abci/types');
const BlockExecutionContext = require('../../blockExecution/BlockExecutionContext');

const proposalStatus = {
  UNKNOWN: 0, // Unknown status. Returning this from the application is always an error.
  ACCEPT: 1, // Status that signals that the application finds the proposal valid.
  REJECT: 2, // Status that signals that the application finds the proposal invalid.
};

/**
 * @param {deliverTx} deliverTx
 * @param {BaseLogger} logger
 * @param {ProposalBlockExecutionContextCollection} proposalBlockExecutionContextCollection
 * @param {beginBlock} beginBlock
 * @param {endBlock} endBlock
 * @param {verifyChainLock} verifyChainLock
 * @param {ExecutionTimer} executionTimer
 * @return {processProposalHandler}
 */
function processProposalHandlerFactory(
  deliverTx,
  logger,
  proposalBlockExecutionContextCollection,
  beginBlock,
  endBlock,
  verifyChainLock,
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
      coreChainLockUpdate,
      round,
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
      const txResult = await deliverTx(tx, round, consensusLogger);

      if (txResult.code === 0) {
        validTxCount += 1;
      } else {
        invalidTxCount += 1;
      }

      txResults.push(txResult);
    }

    const proposalBlockExecutionContext = new BlockExecutionContext();
    proposalBlockExecutionContextCollection.add(round, proposalBlockExecutionContext);

    proposalBlockExecutionContext.setConsensusLogger(consensusLogger);

    const processingFees = proposalBlockExecutionContext.getCumulativeProcessingFee();
    const storageFees = proposalBlockExecutionContext.getCumulativeStorageFee();

    const {
      consensusParamUpdates,
      validatorSetUpdate,
      appHash,
    } = await endBlock(height, round, processingFees, storageFees, consensusLogger);

    if (coreChainLockUpdate) {
      const coreChainLock = new CoreChainLock({
        coreBlockHeight: coreChainLockUpdate.coreBlockHeight,
        coreBlockHash: Buffer.from(coreChainLockUpdate.coreBlockHash),
        signature: Buffer.from(coreChainLockUpdate.signature),
      });

      await verifyChainLock(coreChainLock);
    }

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
