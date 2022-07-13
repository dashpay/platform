const {
  tendermint: {
    abci: {
      ResponseFinalizeBlock,
    },
  },
} = require('@dashevo/abci/types');

/**
 *
 * @return {finalizeBlockHandler}
 * @param {GroveDBStore} groveDBStore
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {beginBlock} beginBlock
 * @param {deliverTx} deliverTx
 * @param {endBlock} endBlock
 * @param {commit} commit
 * @param {BaseLogger} logger
 * @param {ExecutionTimer} executionTimer
 */
function finalizeBlockHandlerFactory(
  groveDBStore,
  blockExecutionContext,
  beginBlock,
  deliverTx,
  endBlock,
  commit,
  logger,
  executionTimer,
) {
  /**
   * @typedef finalizeBlockHandler
   *
   * @param {abci.RequestFinalizeBlock} request
   * @return {Promise<abci.ResponseFinalizeBlock>}
   */
  async function finalizeBlockHandler(request) {
    // Start block execution timer
    executionTimer.clearTimer('blockExecution');

    executionTimer.startTimer('blockExecution');

    const {
      txs,
      decidedLastCommit: lastCommitInfo,
      height,
      time,
      coreChainLockedHeight,
      version,
    } = request;

    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'finalizeBlock',
    });

    consensusLogger.debug('FinalizeBlock ABCI method requested');
    consensusLogger.trace({ abciRequest: request });

    blockExecutionContext.setConsensusLogger(consensusLogger);

    await beginBlock({
      lastCommitInfo, height, coreChainLockedHeight, version, time,
    });

    const txResults = [];

    for (const tx of txs) {
      txResults.push(await deliverTx(tx));
    }

    const endBlockResult = await endBlock(height);
    const commitResult = await commit();

    return new ResponseFinalizeBlock({
      txResults,
      ...commitResult,
      ...endBlockResult,
    });
  }

  return finalizeBlockHandler;
}

module.exports = finalizeBlockHandlerFactory;
