const {
  tendermint: {
    abci: {
      ResponseProcessProposal,
    },
  },
} = require('@dashevo/abci/types');

const statuses = require('./statuses');

const aggregateFees = require('./fees/aggregateFees');

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
   * @param {BaseLogger} consensusLogger
   *
   * @typedef processProposal
   */
  async function processProposal(request, consensusLogger) {
    const {
      height,
      txs,
      coreChainLockedHeight,
      version,
      proposedLastCommit: lastCommitInfo,
      time,
      proposerProTxHash,
      round,
    } = request;

    consensusLogger.info(`Processing a block proposal for height #${height} round #${round}`);

    await beginBlock(
      {
        lastCommitInfo,
        height,
        coreChainLockedHeight,
        version,
        time,
        proposerProTxHash: Buffer.from(proposerProTxHash),
        round,
      },
      consensusLogger,
    );

    const txResults = [];
    const feeResults = [];

    let validTxCount = 0;
    let invalidTxCount = 0;

    for (const tx of txs) {
      const {
        code,
        info,
        fees,
      } = await wrappedDeliverTx(tx, round, consensusLogger);

      if (code === 0) {
        validTxCount += 1;
        // TODO We probably should calculate fees for invalid transitions as well
        feeResults.push(fees);
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
    proposalBlockExecutionContext.setConsensusLogger(consensusLogger);

    const {
      consensusParamUpdates,
      validatorSetUpdate,
      appHash,
    } = await endBlock({
      height,
      round,
      fees: aggregateFees(feeResults),
      coreChainLockedHeight,
    }, consensusLogger);

    const roundExecutionTime = executionTimer.getTimer('roundExecution', true);

    consensusLogger.info(
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
