const {
  tendermint: {
    abci: {
      ResponseBeginBlock,
    },
  },
} = require('@dashevo/abci/types');

const NotSupportedNetworkProtocolVersionError = require('./errors/NotSupportedNetworkProtocolVersionError');
const NetworkProtocolVersionIsNotSetError = require('./errors/NetworkProtocolVersionIsNotSetError');
const timeToMillis = require('../../util/timeToMillis');

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
 * @param {RSAbci} rsAbci
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
  rsAbci,
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

    // Start block execution timer
    executionTimer.clearTimer('blockExecution');

    executionTimer.startTimer('blockExecution');

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

    // Make sure Core has the same height as the network

    await waitForChainLockedHeight(coreChainLockedHeight);

    // Set block execution context

    // in case previous block execution failed in process
    // and not committed. We need to make sure
    // previous context properly reset.
    const previousContext = blockExecutionContextStack.getFirst();
    if (previousContext && previousContext.getHeader().height.equals(height)) {
      // Remove failed block context from the stack
      blockExecutionContextStack.removeFirst();
    }

    blockExecutionContext.reset();

    blockExecutionContext.setConsensusLogger(consensusLogger);

    blockExecutionContext.setHeader(header);

    blockExecutionContext.setLastCommitInfo(lastCommitInfo);

    // Set protocol version to DPP
    dpp.setProtocolVersion(version.app.toNumber());
    transactionalDpp.setProtocolVersion(version.app.toNumber());

    if (await groveDBStore.isTransactionStarted()) {
      await groveDBStore.abortTransaction();
    }

    // Start db transaction for the block
    await groveDBStore.startTransaction();

    // Call RS ABCI

    /**
     * @type {BlockBeginRequest}
     */
    const rsRequest = {
      blockHeight: header.height.toNumber(),
      blockTimeMs: timeToMillis(header.time.seconds, header.time.nanos),
      proposerProTxHash: header.proposerProTxHash,
      validatorSetQuorumHash: Buffer.alloc(32),
    };

    if (previousContext) {
      const previousHeader = previousContext.getHeader();

      rsRequest.previousBlockTimeMs = timeToMillis(
        previousHeader.time.seconds, previousHeader.time.nanos,
      );
    }

    logger.debug(rsRequest, 'Request RS Drive\'s BlockBegin method');

    const rsResponse = await rsAbci.blockBegin(rsRequest, true);

    blockExecutionContext.setEpochInfo(rsResponse.epochInfo);

    const { currentEpochIndex, isEpochChange } = rsResponse;

    if (isEpochChange) {
      const blockTime = timeToMillis(header.time.seconds, header.time.nanos);

      const debugData = {
        currentEpochIndex,
        blockTime,
      };

      if (rsRequest.previousBlockTimeMs) {
        debugData.previousBlockTimeMs = rsRequest.previousBlockTimeMs;
      }

      const blockTimeFormatted = new Date(blockTime).toUTCString();

      consensusLogger.debug(debugData, `Fee epoch #${currentEpochIndex} started on block #${height} at ${blockTimeFormatted}`);
    }

    // Update SML

    const isSimplifiedMasternodeListUpdated = await updateSimplifiedMasternodeList(
      coreChainLockedHeight,
      {
        logger: consensusLogger,
      },
    );

    if (isSimplifiedMasternodeListUpdated) {
      const synchronizeMasternodeIdentitiesResult = await synchronizeMasternodeIdentities(
        coreChainLockedHeight,
      );

      const {
        createdEntities, updatedEntities, removedEntities, fromHeight, toHeight,
      } = synchronizeMasternodeIdentitiesResult;

      consensusLogger.info(
        `Masternode identities are synced for heights from ${fromHeight} to ${toHeight}: ${createdEntities.length} created, ${updatedEntities.length} updated, ${removedEntities.length} removed`,
      );

      if (createdEntities.length > 0 || updatedEntities.length > 0 || removedEntities.length > 0) {
        consensusLogger.trace(
          {
            createdEntities: createdEntities.map((item) => item.toJSON()),
            updatedEntities: updatedEntities.map((item) => item.toJSON()),
            removedEntities: removedEntities.map((item) => item.toJSON()),
          },
          'Synchronized masternode identities',
        );
      }
    }

    consensusLogger.info(`Block begin #${height}`);

    return new ResponseBeginBlock();
  }

  return beginBlockHandler;
}

module.exports = beginBlockHandlerFactory;
