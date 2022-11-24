/**
 * Begin block ABCI
 *
 * @param {BlockExecutionContext} proposalBlockExecutionContext
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
  proposalBlockExecutionContext,
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

    // Call RS ABCI

    const rsRequest = {
      fees: {
        processingFees,
        storageFees,
      },
    };

    consensusLogger.debug(rsRequest, 'Request RS Drive\'s BlockEnd method');

    const transaction = proposalBlockExecutionContext.getTransaction();

    const rsResponse = await rsAbci.blockEnd(rsRequest, transaction);

    consensusLogger.debug(rsResponse, 'RS Drive\'s BlockEnd method response');

    const { currentEpochIndex } = proposalBlockExecutionContext.getEpochInfo();

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
    const appHash = await groveDBStore.getRootHash({ transaction });

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
