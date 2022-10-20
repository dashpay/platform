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
 * @param {GroveDBStore} groveDBStore
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {beginBlock} beginBlock
 * @param {endBlock} endBlock
 * @param {updateConsensusParams} updateConsensusParams
 * @param {rotateValidators} rotateValidators
 * @param {updateCoreChainLock} updateCoreChainLock
 * @return {prepareProposalHandler}
 */
function prepareProposalHandlerFactory(
  deliverTx,
  logger,
  groveDBStore,
  blockExecutionContext,
  beginBlock,
  endBlock,
  updateConsensusParams,
  rotateValidators,
  updateCoreChainLock,
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

    for (const tx of txs) {
      totalSizeBytes += tx.length;

      if (totalSizeBytes > maxTxBytes) {
        break;
      }

      txRecords.push({
        tx,
        action: txAction.UNMODIFIED,
      });
      txResults.push(await deliverTx(tx, consensusLogger));
    }

    blockExecutionContext.setConsensusLogger(consensusLogger);

    await endBlock(height, consensusLogger);

    const consensusParamUpdates = await updateConsensusParams(height, consensusLogger);
    const validatorSetUpdate = await rotateValidators(height, consensusLogger);
    const coreChainLockUpdate = await updateCoreChainLock(consensusLogger);

    const validTxCount = blockExecutionContext.getValidTxCount();
    const invalidTxCount = blockExecutionContext.getInvalidTxCount();

    consensusLogger.info(
      {
        validTxCount,
        invalidTxCount,
      },
      `Prepare proposal #${height} (valid txs = ${validTxCount}, invalid txs = ${invalidTxCount})`,
    );

    const appHash = await groveDBStore.getRootHash({ useTransaction: true });

    consensusLogger.info(
      {
        appHash: appHash.toString('hex').toUpperCase(),
      },
      `Block prepareProposal #${height} with appHash ${appHash.toString('hex').toUpperCase()}`,
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
