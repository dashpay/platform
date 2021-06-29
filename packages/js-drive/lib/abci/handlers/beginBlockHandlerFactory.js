const {
  tendermint: {
    abci: {
      ResponseBeginBlock,
    },
  },
} = require('@dashevo/abci/types');

const NotSupportedProtocolVersionError = require('./errors/NotSupportedProtocolVersionError');

/**
 * Begin Block ABCI Handler
 *
 * @param {BlockExecutionStoreTransactions} blockExecutionStoreTransactions
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContext} previousBlockExecutionContext
 * @param {Number} protocolVersion - Protocol version
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {waitForChainLockedHeight} waitForChainLockedHeight
 * @param {BaseLogger} logger
 *
 * @return {beginBlockHandler}
 */
function beginBlockHandlerFactory(
  blockExecutionStoreTransactions,
  blockExecutionContext,
  previousBlockExecutionContext,
  protocolVersion,
  updateSimplifiedMasternodeList,
  waitForChainLockedHeight,
  logger,
) {
  /**
   * @typedef beginBlockHandler
   *
   * @param {abci.RequestBeginBlock} request
   * @return {Promise<abci.ResponseBeginBlock>}
   */
  async function beginBlockHandler(request) {
    const { header, lastCommitInfo } = request;

    const {
      coreChainLockedHeight,
      height,
      version,
    } = header;

    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'beginBlock',
    });

    consensusLogger.debug('BeginBlock ABCI method requested');
    consensusLogger.trace({ abciRequest: request });

    // in case previous block execution failed in process
    // and not committed. We need to make sure
    // previous context copied and reset.

    previousBlockExecutionContext.reset();
    previousBlockExecutionContext.populate(blockExecutionContext);

    blockExecutionContext.reset();

    blockExecutionContext.setConsensusLogger(consensusLogger);

    blockExecutionContext.setHeader(header);

    blockExecutionContext.setLastCommitInfo(lastCommitInfo);

    await waitForChainLockedHeight(coreChainLockedHeight);

    await updateSimplifiedMasternodeList(coreChainLockedHeight, {
      logger: consensusLogger,
    });

    if (version.app.gt(protocolVersion)) {
      throw new NotSupportedProtocolVersionError(
        version.app,
        protocolVersion,
      );
    }

    if (blockExecutionStoreTransactions.isStarted()) {
      // in case previous block execution failed in process
      // and not commited. We need to make sure
      // previous transactions are aborted.
      await blockExecutionStoreTransactions.abort();
    }

    await blockExecutionStoreTransactions.start();

    consensusLogger.info(`Block begin #${height}`);

    return new ResponseBeginBlock();
  }

  return beginBlockHandler;
}

module.exports = beginBlockHandlerFactory;
