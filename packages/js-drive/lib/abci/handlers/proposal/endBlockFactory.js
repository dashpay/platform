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
   * @param {number} request.height
   * @param {number} request.round
   * @param {
   *    storageFee: number,
   *    processingFee: number,
   *    feeRefunds: Object<string, number>,
   *    feeRefundsSum: number
   * } request.fees
   * @param {number} request.coreChainLockedHeight
   * @param {BaseLogger} contextLogger
   * @return {Promise<{
   *   consensusParamUpdates: ConsensusParams,
   *   validatorSetUpdate: ValidatorSetUpdate,
   *   nextCoreChainLockUpdate: CoreChainLock,
   * }>}
   */
  async function endBlock(
    request,
    contextLogger,
  ) {
    const {
      height,
      round,
      fees,
      coreChainLockedHeight,
    } = request;

    // Call RS ABCI

    const rsRequest = {
      fees,
    };

    contextLogger.debug(rsRequest, 'Request RS Drive\'s BlockEnd method');

    const rsResponse = await rsAbci.blockEnd(rsRequest, true);

    contextLogger.debug(rsResponse, 'RS Drive\'s BlockEnd method response');

    const { currentEpochIndex } = proposalBlockExecutionContext.getEpochInfo();

    const {
      processingFee,
      storageFee,
      feeRefundsSum,
    } = fees;

    if (processingFee > 0 || storageFee > 0) {
      contextLogger.debug({
        currentEpochIndex,
        processingFee,
        storageFee,
        feeRefundsSum,
      }, `${processingFee} processing fees added to epoch #${currentEpochIndex}. ${storageFee} storage fees added to distribution pool. ${feeRefundsSum} credits refunded to identities`);
    }

    if (rsResponse.proposersPaidCount) {
      contextLogger.debug({
        currentEpochIndex,
        proposersPaidCount: rsResponse.proposersPaidCount,
        paidEpochIndex: rsResponse.paidEpochIndex,
      }, `${rsResponse.proposersPaidCount} masternodes were paid for epoch #${rsResponse.paidEpochIndex}`);
    }

    if (rsResponse.refundedEpochsCount) {
      contextLogger.debug({
        currentEpochIndex,
        refundedEpochsCount: rsResponse.refundedEpochsCount,
      }, `${rsResponse.refundedEpochsCount} epochs were refunded`);
    }

    const consensusParamUpdates = await createConsensusParamUpdate(height, round, contextLogger);

    const validatorSetUpdate = await rotateAndCreateValidatorSetUpdate(
      height,
      coreChainLockedHeight,
      round,
      contextLogger,
    );

    const appHash = await groveDBStore.getRootHash({ useTransaction: true });

    executionTimer.stopTimer('roundExecution', true);

    return {
      consensusParamUpdates,
      validatorSetUpdate,
      appHash,
    };
  }

  return endBlock;
}

module.exports = endBlockFactory;
