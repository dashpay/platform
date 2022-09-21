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
      proposerProTxHash,
    } = request;

    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'finalizeBlock',
    });

    consensusLogger.debug('FinalizeBlock ABCI method requested');
    consensusLogger.trace({ abciRequest: request });

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

    const endBlockResult = await endBlock(height, consensusLogger);
    const commitResult = await commit(lastCommitInfo, consensusLogger);

    const blockExecutionTimings = executionTimer.stopTimer('blockExecution');
    const blockHeight = blockExecutionContext.getHeight();

    consensusLogger.trace(
      {
        timings: blockExecutionTimings,
      },
      `Block #${blockHeight} execution took ${blockExecutionTimings} seconds`,
    );

    return new ResponseFinalizeBlock({
      txResults,
      ...commitResult,
      ...endBlockResult,
    });
  }

  return finalizeBlockHandler;
}

module.exports = finalizeBlockHandlerFactory;
