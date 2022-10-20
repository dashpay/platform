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
 * @param {updateConsensusParams} updateConsensusParams
 * @param {rotateValidators} rotateValidators
 * @return {processProposalHandler}
 */
function processProposalHandlerFactory(
  deliverTx,
  logger,
  groveDBStore,
  blockExecutionContext,
  beginBlock,
  endBlock,
  updateConsensusParams,
  rotateValidators,
) {
  /**
   * @typedef processProposalHandler
   * @return {Promise<abci.ResponseProcessProposal>}
   */
  async function processProposalHandler({
    height,
    txs,
    coreChainLockedHeight,
    version,
    proposedLastCommit: lastCommitInfo,
    time,
    proposerProTxHash,
  }) {
    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'processProposal',
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

    const txResults = [];

    for (const tx of txs) {
      txResults.push(await deliverTx(tx, consensusLogger));
    }

    blockExecutionContext.setConsensusLogger(consensusLogger);

    await endBlock(height, consensusLogger);

    const consensusParamUpdates = await updateConsensusParams(height, consensusLogger);
    const validatorSetUpdate = await rotateValidators(height, consensusLogger);

    const validTxCount = blockExecutionContext.getValidTxCount();
    const invalidTxCount = blockExecutionContext.getInvalidTxCount();

    consensusLogger.info(
      {
        validTxCount,
        invalidTxCount,
      },
      `Process proposal #${height} (valid txs = ${validTxCount}, invalid txs = ${invalidTxCount})`,
    );

    const appHash = await groveDBStore.getRootHash({ useTransaction: true });

    consensusLogger.info(
      {
        appHash: appHash.toString('hex').toUpperCase(),
      },
      `Block prepareProposal #${height} with appHash ${appHash.toString('hex').toUpperCase()}`,
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
