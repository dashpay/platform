const {
  tendermint: {
    abci: {
      ResponsePrepareProposal,
    },
  },
} = require('@dashevo/abci/types');

const lodashCloneDeep = require('lodash/cloneDeep');
const addStateTransitionFeesToBlockFees = require('./proposal/fees/addStateTransitionFeesToBlockFees');

const txAction = {
  UNKNOWN: 0, // Unknown action
  UNMODIFIED: 1, // The Application did not modify this transaction.
  ADDED: 2, // The Application added this transaction.
  REMOVED: 3, // The Application wants this transaction removed from the proposal and the mempool.
};

/**
 * @param {deliverTx} wrappedDeliverTx
 * @param {BaseLogger} logger
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 * @param {beginBlock} beginBlock
 * @param {endBlock} endBlock
 * @param {createCoreChainLockUpdate} createCoreChainLockUpdate
 * @param {ExecutionTimer} executionTimer
 * @return {prepareProposalHandler}
 */
function prepareProposalHandlerFactory(
  wrappedDeliverTx,
  logger,
  proposalBlockExecutionContext,
  beginBlock,
  endBlock,
  createCoreChainLockUpdate,
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
      round,
      abciMethod: 'prepareProposal',
    });

    const requestToLog = lodashCloneDeep(request);
    delete requestToLog.txs;

    consensusLogger.debug('PrepareProposal ABCI method requested');
    consensusLogger.trace({ abciRequest: requestToLog });

    consensusLogger.info(`Preparing a block proposal for height #${height} round #${round}`);

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

    let totalSizeBytes = 0;

    const txRecords = [];
    const txResults = [];
    const blockFees = {
      storageFee: 0,
      processingFee: 0,
      refundsPerEpoch: { },
    };

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

      const {
        code,
        info,
        fees,
      } = await wrappedDeliverTx(tx, round, consensusLogger);

      if (code === 0) {
        validTxCount += 1;
        // TODO We probably should calculate fees for invalid transitions as well
        addStateTransitionFeesToBlockFees(blockFees, fees);
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

    const coreChainLockUpdate = await createCoreChainLockUpdate(
      coreChainLockedHeight,
      round,
      consensusLogger,
    );

    const {
      consensusParamUpdates,
      validatorSetUpdate,
      appHash,
    } = await endBlock({
      height,
      round,
      fees: blockFees,
      coreChainLockedHeight,
    }, consensusLogger);

    const roundExecutionTime = executionTimer.getTimer('roundExecution', true);

    const mempoolTxCount = txs.length - validTxCount - invalidTxCount;

    consensusLogger.info(
      {
        roundExecutionTime,
        validTxCount,
        invalidTxCount,
        mempoolTxCount,
      },
      `Prepared block proposal for height #${height} with appHash ${appHash.toString('hex').toUpperCase()}`
      + ` in ${roundExecutionTime} seconds (valid txs = ${validTxCount}, invalid txs = ${invalidTxCount}, mempool txs = ${mempoolTxCount})`,
    );

    proposalBlockExecutionContext.setPrepareProposalResult({
      appHash,
      txResults,
      consensusParamUpdates,
      validatorSetUpdate,
    });

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
