const timeToMillis = require('../../../util/timeToMillis');

/**
 * Begin block ABCI
 *
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {getFeatureFlagForHeight} getFeatureFlagForHeight
 * @param {RSAbci} rsAbci
 *
 * @return {endBlock}
 */
function endBlockFactory(
  blockExecutionContext,
  validatorSet,
  createValidatorSetUpdate,
  getFeatureFlagForHeight,
  rsAbci,
) {
  /**
   * @typedef endBlock
   *
   * @param {number} height
   * @param {BaseLogger} logger
   * @return {Promise<{
   *   consensusParamUpdates: ConsensusParams,
   *   validatorSetUpdate: ValidatorSetUpdate,
   *   nextCoreChainLockUpdate: CoreChainLock,
   * }>}
   */
  async function endBlock(height, logger) {
    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'endBlock',
    });

    consensusLogger.debug('EndBlock ABCI method requested');

    blockExecutionContext.setConsensusLogger(consensusLogger);

    // Call RS ABCI
    const processingFees = blockExecutionContext.getCumulativeProcessingFee();
    const storageFees = blockExecutionContext.getCumulativeStorageFee();

    const rsRequest = {
      fees: {
        processingFees,
        storageFees,
      },
    };

    logger.debug(rsRequest, 'Request RS Drive\'s BlockEnd method');

    const rsResponse = await rsAbci.blockEnd(rsRequest, true);

    logger.debug(rsResponse, 'RS Drive\'s BlockEnd method response');

    const { currentEpochIndex, isEpochChange } = rsResponse;

    if (isEpochChange) {
      const time = blockExecutionContext.getTime();

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
  }

  return endBlock;
}

module.exports = endBlockFactory;
