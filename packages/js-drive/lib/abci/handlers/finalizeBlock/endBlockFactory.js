const {
  tendermint: {
    types: {
      CoreChainLock,
      ConsensusParams,
    },
  },
} = require('@dashevo/abci/types');

const featureFlagTypes = require('@dashevo/feature-flags-contract/lib/featureFlagTypes');

const timeToMillis = require('../../../util/timeToMillis');

/**
 * Begin block ABCI
 *
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @param {LatestCoreChainLock} latestCoreChainLock
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {getFeatureFlagForHeight} getFeatureFlagForHeight
 * @param {RSAbci} rsAbci
 *
 * @return {endBlock}
 */
function endBlockFactory(
  blockExecutionContext,
  blockExecutionContextStack,
  latestCoreChainLock,
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
      abciMethod: 'finalizeBlock#endBlock',
    });

    consensusLogger.debug('EndBlock ABCI method requested');

    blockExecutionContext.setConsensusLogger(consensusLogger);

    const contextVersion = blockExecutionContext.getVersion();
    const contextCoreChainLockedHeight = blockExecutionContext.getCoreChainLockedHeight();
    const lastCommitInfo = blockExecutionContext.getLastCommitInfo();
    const coreChainLock = latestCoreChainLock.getChainLock();

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

      const previousContext = blockExecutionContextStack.getFirst();

      if (previousContext) {
        const previousTime = previousContext.getTime();

        debugData.previousBlockTimeMs = timeToMillis(
          previousTime.seconds, previousTime.nanos,
        );
      }

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

    // Rotate validators

    let validatorSetUpdate;
    const rotationEntropy = Buffer.from(lastCommitInfo.stateSignature);
    if (await validatorSet.rotate(height, coreChainLock.height, rotationEntropy)) {
      validatorSetUpdate = createValidatorSetUpdate(validatorSet);

      const { quorumHash } = validatorSet.getQuorum();

      consensusLogger.debug(
        {
          quorumHash,
        },
        `Validator set switched to ${quorumHash} quorum`,
      );
    }

    // Update Core Chain Locks

    let nextCoreChainLockUpdate;
    if (coreChainLock && coreChainLock.height > contextCoreChainLockedHeight) {
      nextCoreChainLockUpdate = new CoreChainLock({
        coreBlockHeight: coreChainLock.height,
        coreBlockHash: coreChainLock.blockHash,
        signature: coreChainLock.signature,
      });

      consensusLogger.trace(
        {
          nextCoreChainLockHeight: coreChainLock.height,
        },
        `Provide next chain lock for Core height ${coreChainLock.height}`,
      );
    }

    // Update consensus params feature flag

    const updateConsensusParamsFeatureFlag = await getFeatureFlagForHeight(
      featureFlagTypes.UPDATE_CONSENSUS_PARAMS,
      height,
      true,
    );

    let consensusParamUpdates;
    if (updateConsensusParamsFeatureFlag) {
      // Use previous version if we aren't going to update it
      let version = {
        appVersion: contextVersion.app,
      };

      if (updateConsensusParamsFeatureFlag.get('version')) {
        version = updateConsensusParamsFeatureFlag.get('version');
      }

      consensusParamUpdates = new ConsensusParams({
        block: updateConsensusParamsFeatureFlag.get('block'),
        evidence: updateConsensusParamsFeatureFlag.get('evidence'),
        version,
      });

      consensusLogger.info(
        {
          consensusParamUpdates,
        },
        'Update consensus params',
      );
    }

    const validTxCount = blockExecutionContext.getValidTxCount();
    const invalidTxCount = blockExecutionContext.getInvalidTxCount();

    consensusLogger.info(
      {
        validTxCount,
        invalidTxCount,
      },
      `Block end #${height} (valid txs = ${validTxCount}, invalid txs = ${invalidTxCount})`,
    );

    return {
      consensusParamUpdates,
      validatorSetUpdate,
      nextCoreChainLockUpdate,
    };
  }

  return endBlock;
}

module.exports = endBlockFactory;
