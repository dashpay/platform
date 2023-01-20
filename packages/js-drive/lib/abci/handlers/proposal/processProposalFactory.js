const {
  tendermint: {
    abci: {
      ResponseProcessProposal,
    },
  },
} = require('@dashevo/abci/types');

const statuses = require('./statuses');

const addToFeeTxResults = require('./fees/addToFeeTxResults');

/**
 *
 * @param {deliverTx} wrappedDeliverTx
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 * @param {beginBlock} beginBlock
 * @param {endBlock} endBlock
 * @param {ExecutionTimer} executionTimer
 *
 * @return {processProposal}
 */
function processProposalFactory(
  wrappedDeliverTx,
  proposalBlockExecutionContext,
  beginBlock,
  endBlock,
  executionTimer,
) {
  /**
   * @param {abci.RequestProcessProposal} request
   * @param {BaseLogger} contextLogger
   *
   * @typedef processProposal
   */
  async function processProposal(request, contextLogger) {
    const {
      height,
      txs,
      coreChainLockedHeight,
      version,
      proposedLastCommit: lastCommitInfo,
      time,
      proposerProTxHash,
      round,
      quorumHash,
    } = request;

    contextLogger.info(`Processing a block proposal for height #${height} round #${round}`);

    await beginBlock(
      {
        lastCommitInfo,
        height,
        coreChainLockedHeight,
        version,
        time,
        proposerProTxHash: Buffer.from(proposerProTxHash),
        round,
        quorumHash,
      },
      contextLogger,
    );

    const txResults = [];
    const feeResults = {
      storageFee: 0,
      processingFee: 0,
      feeRefunds: { },
      feeRefundsSum: 0,
    };

    let validTxCount = 0;
    let invalidTxCount = 0;

    for (const tx of txs) {
      const {
        code,
        info,
        fees,
      } = await wrappedDeliverTx(tx, round, contextLogger);

      if (code === 0) {
        validTxCount += 1;
        // TODO We should calculate fees for invalid transitions as well
        addToFeeTxResults(feeResults, fees);
      } else {
        invalidTxCount += 1;
      }

      const txResult = { code };

      if (info) {
        txResult.info = info;
      }

      txResults.push(txResult);
    }

    // Revert consensus logger after deliverTx
    proposalBlockExecutionContext.setContextLogger(contextLogger);

    const {
      consensusParamUpdates,
      validatorSetUpdate,
      appHash,
    } = await endBlock({
      height,
      round,
      fees: feeResults,
      coreChainLockedHeight,
    }, contextLogger);

    const roundExecutionTime = executionTimer.getTimer('roundExecution', true);

    contextLogger.info(
      {
        roundExecutionTime,
        validTxCount,
        invalidTxCount,
      },
      `Processed proposal #${height} with appHash ${appHash.toString('hex').toUpperCase()}`
      + ` in ${roundExecutionTime} seconds (valid txs = ${validTxCount}, invalid txs = ${invalidTxCount})`,
    );

    return new ResponseProcessProposal({
      status: statuses.ACCEPT,
      appHash,
      txResults,
      consensusParamUpdates,
      validatorSetUpdate,
    });
  }

  return processProposal;
}

module.exports = processProposalFactory;
