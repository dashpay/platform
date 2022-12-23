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
      fees,
      coreChainLockedHeight,
    } = request;

    // Call RS ABCI

    const rsRequest = {
      fees,
    };

    consensusLogger.debug(rsRequest, 'Request RS Drive\'s BlockEnd method');

    const rsResponse = await rsAbci.blockEnd(rsRequest, true);

    consensusLogger.debug(rsResponse, 'RS Drive\'s BlockEnd method response');

    const { currentEpochIndex } = proposalBlockExecutionContext.getEpochInfo();

    const {
      processingFee,
      storageFee,
      feeRefundsSum,
    } = fees;

    if (processingFee > 0 || storageFee > 0) {
      consensusLogger.debug({
        currentEpochIndex,
        processingFee,
        storageFee,
        feeRefundsSum,
      }, `${processingFee} processing fees added to epoch #${currentEpochIndex}. ${storageFee} storage fees added to distribution pool. ${feeRefundsSum} credits refunded to identities`);
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
