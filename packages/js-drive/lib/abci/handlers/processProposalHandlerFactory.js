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

const proposalStatus = {
  UNKNOWN: 0, // Unknown status. Returning this from the application is always an error.
  ACCEPT: 1, // Status that signals that the application finds the proposal valid.
  REJECT: 2, // Status that signals that the application finds the proposal invalid.
};

/**
 * @param {deliverTx} wrappedDeliverTx
 * @param {BaseLogger} logger
 * @param {ProposalBlockExecutionContextCollection} proposalBlockExecutionContextCollection
 * @param {beginBlock} beginBlock
 * @param {endBlock} endBlock
 * @param {verifyChainLock} verifyChainLock
 * @return {processProposalHandler}
 */
function processProposalHandlerFactory(
  wrappedDeliverTx,
  logger,
  proposalBlockExecutionContextCollection,
  beginBlock,
  endBlock,
  verifyChainLock,
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

    const proposalBlockExecutionContext = proposalBlockExecutionContextCollection.get(round);

    const txResults = [];
    let validTxCount = 0;
    let invalidTxCount = 0;
    let storageFeesTotal = 0;
    let processingFeesTotal = 0;

    for (const tx of txs) {
      const {
        code,
        info,
        processingFees,
        storageFees,
      } = await wrappedDeliverTx(tx, round, consensusLogger);

      if (code === 0) {
        validTxCount += 1;
        // TODO We probably should calculate fees for invalid transitions as well
        storageFeesTotal += storageFees;
        processingFeesTotal += processingFees;
      } else {
        invalidTxCount += 1;
      }

      const txResult = { code };

      if (info) {
        txResult.info = info;
      }

      txResults.push(txResult);
    }

    proposalBlockExecutionContext.setConsensusLogger(consensusLogger);

    const {
      consensusParamUpdates,
      validatorSetUpdate,
      appHash,
    } = await endBlock({
      height, round, processingFees: processingFeesTotal, storageFees: storageFeesTotal,
    }, consensusLogger);

    if (coreChainLockUpdate) {
      const coreChainLock = new CoreChainLock({
        coreBlockHeight: coreChainLockUpdate.coreBlockHeight,
        coreBlockHash: Buffer.from(coreChainLockUpdate.coreBlockHash),
        signature: Buffer.from(coreChainLockUpdate.signature),
      });

      await verifyChainLock(coreChainLock);
    }

    consensusLogger.info(
      {
        validTxCount,
        invalidTxCount,
      },
      `Process proposal #${height} with appHash ${appHash.toString('hex').toUpperCase()}`
      + ` (valid txs = ${validTxCount}, invalid txs = ${invalidTxCount})`,
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
