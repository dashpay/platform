const { hash } = require('@dashevo/dpp/lib/util/hash');

const NotSupportedNetworkProtocolVersionError = require('../errors/NotSupportedNetworkProtocolVersionError');
const NetworkProtocolVersionIsNotSetError = require('../errors/NetworkProtocolVersionIsNotSetError');

const timeToMillis = require('../../../util/timeToMillis');
const BlockExecutionContext = require('../../../blockExecution/BlockExecutionContext');

/**
 * Begin Block
 *
 * @param {GroveDBStore} groveDBStore
 * @param {BlockExecutionContext} latestBlockExecutionContext
 * @param {ProposalBlockExecutionContextCollection} proposalBlockExecutionContextCollection
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
  proposalBlockExecutionContextCollection,
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
   * @param {ILastCommitInfo} [request.lastCommitInfo]
   * @param {Long} [request.height]
   * @param {number} [request.coreChainLockedHeight]
   * @param {IConsensus} [request.version]
   * @param {ITimestamp} [request.time]
   * @param {Buffer} [request.proposerProTxHash]
   * @param {BaseLogger} consensusLogger
   *
   * @return {Promise<void>}
   */
  async function beginBlock(request, consensusLogger) {
    const {
      lastCommitInfo,
      height,
      coreChainLockedHeight,
      version,
      time,
      proposerProTxHash,
      round,
    } = request;

    if (proposalBlockExecutionContextCollection.isEmpty()) {
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

    // Set block execution context

    const proposalBlockExecutionContext = new BlockExecutionContext();

    proposalBlockExecutionContextCollection.add(round, proposalBlockExecutionContext);

    // Set block execution context params
    proposalBlockExecutionContext.setConsensusLogger(consensusLogger);
    proposalBlockExecutionContext.setHeight(height);
    proposalBlockExecutionContext.setVersion(version);
    proposalBlockExecutionContext.setTime(time);
    proposalBlockExecutionContext.setCoreChainLockedHeight(coreChainLockedHeight);
    proposalBlockExecutionContext.setLastCommitInfo(lastCommitInfo);

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
      // TODO replace with real value
      validatorSetQuorumHash: Buffer.alloc(32),
    };

    if (!latestBlockExecutionContext.isEmpty()) {
      const previousTime = latestBlockExecutionContext.getTime();

      rsRequest.previousBlockTimeMs = timeToMillis(
        previousTime.seconds, previousTime.nanos,
      );
    }

    consensusLogger.debug(rsRequest, 'Request RS Drive\'s BlockBegin method');

    const { unsignedWithdrawalTransactions } = await rsAbci.blockBegin(rsRequest, true);

    const withdrawalTransactionsMap = (unsignedWithdrawalTransactions || []).reduce(
      (map, transactionBytes) => ({
        ...map,
        [hash(transactionBytes).toString('hex')]: transactionBytes,
      }),
      {},
    );

    proposalBlockExecutionContext.setWithdrawalTransactionsMap(withdrawalTransactionsMap);

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
