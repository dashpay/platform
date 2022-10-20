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
 * @param {commit} commit
 * @param {BaseLogger} logger
 * @param {ExecutionTimer} executionTimer
 */
function finalizeBlockHandlerFactory(
  groveDBStore,
  blockExecutionContext,
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
      decidedLastCommit: lastCommitInfo,
      height,
      time,
      coreChainLockedHeight,
    } = request;

    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'finalizeBlock',
    });

    consensusLogger.debug('FinalizeBlock ABCI method requested');
    consensusLogger.trace({ abciRequest: request });

    blockExecutionContext.setPreviousTime(time);
    blockExecutionContext.setPreviousHeight(height);
    blockExecutionContext.setPreviousCoreChainLockedHeight(coreChainLockedHeight);

    await commit(lastCommitInfo, consensusLogger);

    const blockExecutionTimings = executionTimer.stopTimer('blockExecution');
    const blockHeight = blockExecutionContext.getHeight();

    consensusLogger.trace(
      {
        timings: blockExecutionTimings,
      },
      `Block #${blockHeight} execution took ${blockExecutionTimings} seconds`,
    );

    return new ResponseFinalizeBlock();
  }

  return finalizeBlockHandler;
}

module.exports = finalizeBlockHandlerFactory;
