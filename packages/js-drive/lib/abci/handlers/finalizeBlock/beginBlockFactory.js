const { hash } = require('@dashevo/dpp/lib/util/hash');

const NotSupportedNetworkProtocolVersionError = require('../errors/NotSupportedNetworkProtocolVersionError');
const NetworkProtocolVersionIsNotSetError = require('../errors/NetworkProtocolVersionIsNotSetError');

const timeToMillis = require('../../../util/timeToMillis');

/**
 * Begin Block
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
 * @param {RSAbci} rsAbci
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
  rsAbci,
) {
  /**
   * @typedef beginBlock
   * @param {Object} request
   * @param {ILastCommitInfo} [request.lastCommitInfo]
   * @param {Long} [request.height]
   * @param {number} [request.coreChainLockedHeight]
   * @param {IConsensus} [request.version]
   * @param {ITimestamp} [request.time]
   * @param {Buffer} [request.proposerProTxHash]
   * @param {BaseLogger} logger
   *
   * @return {Promise<void>}
   */
  async function beginBlock(request, logger) {
    const {
      lastCommitInfo,
      height,
      coreChainLockedHeight,
      version,
      time,
      proposerProTxHash,
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
    const previousContext = blockExecutionContextStack.getFirst();
    if (
      previousContext
      && previousContext.getHeight()
      && previousContext.getHeight().equals(height)
    ) {
      // Remove failed block context from the stack
      blockExecutionContextStack.removeFirst();
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

    // Call RS ABCI

    /**
     * @type {BlockBeginRequest}
     */
    const rsRequest = {
      blockHeight: height.toNumber(),
      blockTimeMs: timeToMillis(time.seconds, time.nanos),
      proposerProTxHash,
      validatorSetQuorumHash: Buffer.alloc(32),
    };

    if (previousContext) {
      const previousTime = previousContext.getTime();

      rsRequest.previousBlockTimeMs = timeToMillis(
        previousTime.seconds, previousTime.nanos,
      );
    }

    logger.debug(rsRequest, 'Request RS Drive\'s BlockBegin method');

    const { unsignedWithdrawalTransactions } = await rsAbci.blockBegin(rsRequest, true);

    const withdrawalTransactionsMap = (unsignedWithdrawalTransactions || []).reduce(
      (map, transactionBytes) => ({
        ...map,
        [hash(transactionBytes).toString('hex')]: transactionBytes,
      }),
      {},
    );

    blockExecutionContext.setWithdrawalTransactionsMap(withdrawalTransactionsMap);

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
  }

  return beginBlock;
}

module.exports = beginBlockFactory;
