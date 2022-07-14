const NotSupportedNetworkProtocolVersionError = require('../errors/NotSupportedNetworkProtocolVersionError');
const NetworkProtocolVersionIsNotSetError = require('../errors/NetworkProtocolVersionIsNotSetError');

/**
 * Begin Block ABCI
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
 *
 * @return {beginBlock}
 */
function beginBlockFactory(
  groveDBStore,
  blockExecutionContext,
  blockExecutionContextStack,
  latestProtocolVersion,
  dpp,
  transactionalDpp,
  updateSimplifiedMasternodeList,
  waitForChainLockedHeight,
  synchronizeMasternodeIdentities,
) {
  /**
   * @typedef beginBlock
   * @param {Object} request
   * @param {ILastCommitInfo} [request.lastCommitInfo]
   * @param {Long} [request.height]
   * @param {number} [request.coreChainLockedHeight]
   * @param {IConsensus} [request.version]
   * @param {ITimestamp} [request.time]
   * @param {BaseLogger} logger
   *
   * @return {Promise<void>}
   */
  async function beginBlock(request, logger) {
    const {
      lastCommitInfo, height, coreChainLockedHeight, version, time,
    } = request;

    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'finalizeBlock#beginBlock',
    });

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
    const contextHeight = blockExecutionContext.getHeight();
    if (contextHeight && contextHeight.equals(height)) {
      // Remove failed block context from the stack
      const latestContext = blockExecutionContextStack.getLatest();
      const latestContextHeight = latestContext.getHeight();

      if (latestContextHeight.equals(height)) {
        blockExecutionContextStack.removeLatest();
      }
    }

    blockExecutionContext.reset();

    // Set block execution context params
    blockExecutionContext.setConsensusLogger(consensusLogger);
    blockExecutionContext.setHeight(height);
    blockExecutionContext.setVersion(version);
    blockExecutionContext.setTime(time);
    blockExecutionContext.setCoreChainLockedHeight(coreChainLockedHeight);
    blockExecutionContext.setLastCommitInfo(lastCommitInfo);

    // Set protocol version to DPP
    dpp.setProtocolVersion(version.app.toNumber());
    transactionalDpp.setProtocolVersion(version.app.toNumber());

    if (await groveDBStore.isTransactionStarted()) {
      await groveDBStore.abortTransaction();
    }

    // Start db transaction for the block
    await groveDBStore.startTransaction();

    const isSimplifiedMasternodeListUpdated = await updateSimplifiedMasternodeList(
      coreChainLockedHeight, {
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

      consensusLogger.trace(
        {
          createdEntities: createdEntities.map((item) => item.toJSON()),
          updatedEntities: updatedEntities.map((item) => item.toJSON()),
          removedEntities: removedEntities.map((item) => item.toJSON()),
        },
        'Synchronized masternode identities',
      );
    }

    consensusLogger.info(`Block begin #${height}`);
  }

  return beginBlock;
}

module.exports = beginBlockFactory;
