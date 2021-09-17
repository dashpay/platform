const {
  tendermint: {
    abci: {
      ResponseBeginBlock,
    },
  },
} = require('@dashevo/abci/types');

const NotSupportedNetworkProtocolVersionError = require('./errors/NotSupportedProtocolVersionError');
const NetworkProtocolVersionIsNotSetError = require('./errors/NetworkProtocolVersionIsNotSetError');

/**
 * Begin Block ABCI Handler
 *
 * @param {BlockExecutionStoreTransactions} blockExecutionStoreTransactions
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContext} previousBlockExecutionContext
 * @param {Long} latestProtocolVersion
 * @param {DashPlatformProtocol} dpp
 * @param {DashPlatformProtocol} transactionalDpp
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
  latestProtocolVersion,
  dpp,
  transactionalDpp,
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

    // Validate protocol version

    if (version.app.eq(0)) {
      throw new NetworkProtocolVersionIsNotSetError();
    }

    if (version.app.gt(latestProtocolVersion)) {
      throw new NotSupportedNetworkProtocolVersionError(
        version.app,
        latestProtocolVersion,
      );
    }

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

    if (blockExecutionStoreTransactions.isStarted()) {
      // in case previous block execution failed in process
      // and not commited. We need to make sure
      // previous transactions are aborted.
      await blockExecutionStoreTransactions.abort();
    }

    await blockExecutionStoreTransactions.start();

    dpp.setProtocolVersion(version.app.toNumber());
    transactionalDpp.setProtocolVersion(version.app.toNumber());

    consensusLogger.info(`Block begin #${height}`);

    return new ResponseBeginBlock();
  }

  return beginBlockHandler;
}

module.exports = beginBlockHandlerFactory;
