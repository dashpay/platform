const {
  tendermint: {
    abci: {
      ResponseBeginBlock,
    },
  },
} = require('@dashevo/abci/types');

const NotSupportedNetworkProtocolVersionError = require('./errors/NotSupportedNetworkProtocolVersionError');
const NetworkProtocolVersionIsNotSetError = require('./errors/NetworkProtocolVersionIsNotSetError');

/**
 * Begin Block ABCI Handler
 *
 * @param {GroveDBStore} groveDBStore
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @param {Long} latestProtocolVersion
 * @param {DashPlatformProtocol} dpp
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {waitForChainLockedHeight} waitForChainLockedHeight
 * @param {synchronizeMasternodeIdentities} synchronizeMasternodeIdentities
 * @param {BaseLogger} logger
 * @param {ExecutionTimer} executionTimer
 *
 * @return {beginBlockHandler}
 */
function beginBlockHandlerFactory(
  groveDBStore,
  blockExecutionContext,
  blockExecutionContextStack,
  latestProtocolVersion,
  dpp,
  transactionalDpp,
  updateSimplifiedMasternodeList,
  waitForChainLockedHeight,
  synchronizeMasternodeIdentities,
  logger,
  executionTimer,
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

    // Start block execution timer

    if (executionTimer.isStarted('blockExecution')) {
      executionTimer.endTimer('blockExecution');
    }

    executionTimer.startTimer('blockExecution');

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

    // Make sure Core has the same height as the network

    await waitForChainLockedHeight(coreChainLockedHeight);

    // Set block execution context

    // in case previous block execution failed in process
    // and not committed. We need to make sure
    // previous context properly reset.
    const contextHeader = blockExecutionContext.getHeader();
    if (contextHeader && contextHeader.height.equals(height)) {
      if (await groveDBStore.isTransactionStarted()) {
        await groveDBStore.abortTransaction();
      }

      // Remove failed block context from the stack
      const latestContext = blockExecutionContextStack.getLatest();
      const latestContextHeader = latestContext.getHeader();

      if (latestContextHeader.height.equals(height)) {
        blockExecutionContextStack.removeLatest();
      }
    }

    blockExecutionContext.reset();

    blockExecutionContext.setConsensusLogger(consensusLogger);

    blockExecutionContext.setHeader(header);

    blockExecutionContext.setLastCommitInfo(lastCommitInfo);

    // Set protocol version to DPP
    dpp.setProtocolVersion(version.app.toNumber());
    transactionalDpp.setProtocolVersion(version.app.toNumber());

    // Start db transaction for the block
    await groveDBStore.startTransaction();

    const isSimplifiedMasternodeListUpdated = await updateSimplifiedMasternodeList(
      coreChainLockedHeight, {
        logger: consensusLogger,
      },
    );

    if (isSimplifiedMasternodeListUpdated) {
      await synchronizeMasternodeIdentities(coreChainLockedHeight);
    }

    consensusLogger.info(`Block begin #${height}`);

    return new ResponseBeginBlock();
  }

  return beginBlockHandler;
}

module.exports = beginBlockHandlerFactory;
