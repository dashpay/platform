const { hash } = require('@dashevo/dpp/lib/util/hash');

const NotSupportedNetworkProtocolVersionError = require('../errors/NotSupportedNetworkProtocolVersionError');
const NetworkProtocolVersionIsNotSetError = require('../errors/NetworkProtocolVersionIsNotSetError');

const BlockInfo = require('../../../blockExecution/BlockInfo');
const protoTimestampToMillis = require('../../../util/protoTimestampToMillis');

/**
 * Begin Block
 *
 * @param {GroveDBStore} groveDBStore
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 * @param {Long} latestProtocolVersion
 * @param {DashPlatformProtocol} dpp
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {waitForChainLockedHeight} waitForChainLockedHeight
 * @param {synchronizeMasternodeIdentities} synchronizeMasternodeIdentities
 * @param {RSAbci} rsAbci
 * @param {ExecutionTimer} executionTimer
 *
 * @return {beginBlock}
 */
function beginBlockFactory(
  groveDBStore,
  latestBlockExecutionContext,
  proposalBlockExecutionContext,
  latestProtocolVersion,
  dpp,
  transactionalDpp,
  updateSimplifiedMasternodeList,
  waitForChainLockedHeight,
  synchronizeMasternodeIdentities,
  rsAbci,
  executionTimer,
) {
  /**
   * @typedef beginBlock
   * @param {Object} request
   * @param {ILastCommitInfo} request.lastCommitInfo
   * @param {Long} request.height
   * @param {number} request.coreChainLockedHeight
   * @param {IConsensus} request.version
   * @param {ITimestamp} request.time
   * @param {Buffer} request.proposerProTxHash
   * @param {BaseLogger} contextLogger
   *
   * @return {Promise<void>}
   */
  async function beginBlock(request, contextLogger) {
    const {
      lastCommitInfo,
      height,
      coreChainLockedHeight,
      version,
      time,
      proposerProTxHash,
      round,
    } = request;

    if (proposalBlockExecutionContext.isEmpty()) {
      executionTimer.clearTimer('blockExecution');
      executionTimer.startTimer('blockExecution');
    }

    executionTimer.clearTimer('roundExecution');
    executionTimer.startTimer('roundExecution');

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

    // Reset block execution context
    proposalBlockExecutionContext.reset();

    proposalBlockExecutionContext.setContextLogger(contextLogger);
    proposalBlockExecutionContext.setHeight(height);
    proposalBlockExecutionContext.setVersion(version);
    proposalBlockExecutionContext.setRound(round);
    proposalBlockExecutionContext.setTimeMs(protoTimestampToMillis(time));
    proposalBlockExecutionContext.setCoreChainLockedHeight(coreChainLockedHeight);
    proposalBlockExecutionContext.setLastCommitInfo(lastCommitInfo);

    // Set protocol version to DPP
    dpp.setProtocolVersion(version.app.toNumber());
    transactionalDpp.setProtocolVersion(version.app.toNumber());

    // Restart transaction if already started
    if (await groveDBStore.isTransactionStarted()) {
      await groveDBStore.abortTransaction();
    }

    await groveDBStore.startTransaction();

    // Call RS ABCI

    /**
     * @type {BlockBeginRequest}
     */
    const rsRequest = {
      blockHeight: height.toNumber(),
      blockTimeMs: proposalBlockExecutionContext.getTimeMs(),
      proposerProTxHash,
      // TODO replace with real value
      validatorSetQuorumHash: Buffer.alloc(32),
    };

    if (!latestBlockExecutionContext.isEmpty()) {
      rsRequest.previousBlockTimeMs = latestBlockExecutionContext.getTimeMs();
    }

    contextLogger.debug(rsRequest, 'Request RS Drive\'s BlockBegin method');

    const rsResponse = await rsAbci.blockBegin(rsRequest, true);

    const withdrawalTransactionsMap = (rsResponse.unsignedWithdrawalTransactions || []).reduce(
      (map, transactionBytes) => ({
        ...map,
        [hash(transactionBytes).toString('hex')]: transactionBytes,
      }),
      {},
    );

    proposalBlockExecutionContext.setWithdrawalTransactionsMap(withdrawalTransactionsMap);
    proposalBlockExecutionContext.setEpochInfo(rsResponse.epochInfo);

    const { currentEpochIndex, isEpochChange } = rsResponse;

    if (isEpochChange) {
      const debugData = {
        currentEpochIndex,
        blockTime: proposalBlockExecutionContext.getTimeMs(),
      };

      if (rsRequest.previousBlockTimeMs) {
        debugData.previousBlockTimeMs = rsRequest.previousBlockTimeMs;
      }

      const blockTimeFormatted = new Date(proposalBlockExecutionContext.getTimeMs()).toUTCString();

      contextLogger.info(debugData, `Epoch #${currentEpochIndex} started on block #${height} at ${blockTimeFormatted}`);
    }

    // Update SML
    const isSimplifiedMasternodeListUpdated = await updateSimplifiedMasternodeList(
      coreChainLockedHeight,
      {
        logger: contextLogger,
      },
    );

    if (isSimplifiedMasternodeListUpdated) {
      const blockInfo = BlockInfo.createFromBlockExecutionContext(proposalBlockExecutionContext);

      const synchronizeMasternodeIdentitiesResult = await synchronizeMasternodeIdentities(
        coreChainLockedHeight,
        blockInfo,
      );

      const {
        createdEntities, updatedEntities, removedEntities, fromHeight, toHeight,
      } = synchronizeMasternodeIdentitiesResult;

      contextLogger.info(
        `Masternode identities are synced for heights from ${fromHeight} to ${toHeight}: ${createdEntities.length} created, ${updatedEntities.length} updated, ${removedEntities.length} removed`,
      );

      if (createdEntities.length > 0 || updatedEntities.length > 0 || removedEntities.length > 0) {
        contextLogger.trace(
          {
            createdEntities: createdEntities.map((item) => item.toJSON()),
            updatedEntities: updatedEntities.map((item) => item.toJSON()),
            removedEntities: removedEntities.map((item) => item.toJSON()),
          },
          'Synchronized masternode identities',
        );
      }
    }
  }

  return beginBlock;
}

module.exports = beginBlockFactory;
