const timeToMillis = require('../../../util/timeToMillis');

/**
 * Begin block ABCI
 *
 * @param {ProposalBlockExecutionContextCollection} proposalBlockExecutionContextCollection
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {getFeatureFlagForHeight} getFeatureFlagForHeight
 * @param {RSAbci} rsAbci
 * @param {createConsensusParamUpdate} createConsensusParamUpdate
 * @param {rotateAndCreateValidatorSetUpdate} rotateAndCreateValidatorSetUpdate
 * @param {GroveDBStore} groveDBStore
 * @param {ExecutionTimer} executionTimer
 *
 * @return {endBlock}
 */
function endBlockFactory(
  proposalBlockExecutionContextCollection,
  validatorSet,
  createValidatorSetUpdate,
  getFeatureFlagForHeight,
  createConsensusParamUpdate,
  rotateAndCreateValidatorSetUpdate,
  rsAbci,
  groveDBStore,
  executionTimer,
) {
  /**
   * @typedef endBlock
   *
   * @param {Object} request
   * @param {number} [request.height]
   * @param {number} [request.round]
   * @param {number} [request.processingFees]
   * @param {number} [request.storageFees]
   * @param {number} [request.coreChainLockedHeight]
   * @param {BaseLogger} consensusLogger
   * @return {Promise<{
   *   consensusParamUpdates: ConsensusParams,
   *   validatorSetUpdate: ValidatorSetUpdate,
   *   nextCoreChainLockUpdate: CoreChainLock,
   * }>}
   */
  async function endBlock(
    request,
    consensusLogger,
  ) {
    const {
      height,
      round,
      processingFees,
      storageFees,
      coreChainLockedHeight,
    } = request;

    consensusLogger.debug('EndBlock ABCI method requested');
    const proposalBlockExecutionContext = proposalBlockExecutionContextCollection.get(round);

    // Call RS ABCI

    const rsRequest = {
      fees: {
        processingFees,
        storageFees,
      },
    };

    consensusLogger.debug(rsRequest, 'Request RS Drive\'s BlockEnd method');

    const rsResponse = await rsAbci.blockEnd(rsRequest, true);

    consensusLogger.debug(rsResponse, 'RS Drive\'s BlockEnd method response');

    const { currentEpochIndex, isEpochChange } = rsResponse;

    if (isEpochChange) {
      const time = proposalBlockExecutionContext.getTime();

      const blockTime = timeToMillis(time.seconds, time.nanos);

      const debugData = {
        currentEpochIndex,
        blockTime,
      };

      const blockTimeFormatted = new Date(blockTime).toUTCString();

      consensusLogger.debug(debugData, `Fee epoch #${currentEpochIndex} started on block #${height} at ${blockTimeFormatted}`);
    }

    if (processingFees > 0 || storageFees > 0) {
      consensusLogger.debug({
        currentEpochIndex,
        processingFees,
        storageFees,
      }, `${processingFees} processing fees added to epoch #${currentEpochIndex}. ${storageFees} storage fees added to distribution pool`);
    }

    if (rsResponse.proposersPaidCount) {
      consensusLogger.debug({
        currentEpochIndex,
        proposersPaidCount: rsResponse.proposersPaidCount,
        paidEpochIndex: rsResponse.paidEpochIndex,
      }, `${rsResponse.proposersPaidCount} masternodes were paid for epoch #${rsResponse.paidEpochIndex}`);
    }

    const consensusParamUpdates = await createConsensusParamUpdate(height, round, consensusLogger);
    const validatorSetUpdate = await rotateAndCreateValidatorSetUpdate(
      height,
      coreChainLockedHeight,
      round,
      consensusLogger,
    );
    const appHash = await groveDBStore.getRootHash({ useTransaction: true });

    const prepareProposalTimings = executionTimer.stopTimer('roundExecution');

    consensusLogger.info(
      `Round execution took ${prepareProposalTimings} seconds`,
    );

    return {
      consensusParamUpdates,
      validatorSetUpdate,
      appHash,
    };
  }

  return endBlock;
}

module.exports = endBlockFactory;
