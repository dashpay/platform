const {
  tendermint: {
    abci: {
      ResponseEndBlock,
    },
    types: {
      CoreChainLock,
      ConsensusParams,
    },
  },
} = require('@dashevo/abci/types');

const featureFlagTypes = require('@dashevo/feature-flags-contract/lib/featureFlagTypes');

const timeToMillis = require('../../util/timeToMillis');

/**
 * Begin block ABCI handler
 *
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BlockExecutionContextStack} blockExecutionContextStack
 * @param {LatestCoreChainLock} latestCoreChainLock
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {BaseLogger} logger
 * @param {getFeatureFlagForHeight} getFeatureFlagForHeight
 * @param {RSAbci} rsAbci
 *
 * @return {endBlockHandler}
 */
function endBlockHandlerFactory(
  blockExecutionContext,
  blockExecutionContextStack,
  latestCoreChainLock,
  validatorSet,
  createValidatorSetUpdate,
  logger,
  getFeatureFlagForHeight,
  rsAbci,
) {
  /**
   * @typedef endBlockHandler
   *
   * @param {abci.RequestEndBlock} request
   * @return {Promise<abci.ResponseEndBlock>}
   */
  async function endBlockHandler(request) {
    const { height } = request;

    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'endBlock',
    });

    consensusLogger.debug('EndBlock ABCI method requested');

    blockExecutionContext.setConsensusLogger(consensusLogger);

    const header = blockExecutionContext.getHeader();
    const lastCommitInfo = blockExecutionContext.getLastCommitInfo();
    const coreChainLock = latestCoreChainLock.getChainLock();

    // Call RS ABCI
    const processingFees = blockExecutionContext.getCumulativeProcessingFee();
    const storageFees = blockExecutionContext.getCumulativeStorageFee();

    const rsResponse = await rsAbci.blockEnd({
      fees: {
        processingFees,
        storageFees,
      },
    }, true);

    const { currentEpochIndex, isEpochChange } = rsResponse;

    if (isEpochChange) {
      const latestContext = blockExecutionContextStack.getLatest();

      const blockTime = timeToMillis(header.time.seconds, header.time.nanos);

      const debugData = {
        currentEpochIndex,
        blockTime,
      };

      if (latestContext) {
        const latestHeader = latestContext.getHeader();
        debugData.previousBlockTimeMs = timeToMillis(
          latestHeader.time.seconds, latestHeader.time.nanos,
        );
      }

      consensusLogger.debug(debugData, `Fee epoch #${currentEpochIndex} started on block time ${blockTime}`);
    }

    consensusLogger.debug({
      currentEpochIndex,
      processingFees,
      storageFees,
    }, `${processingFees} processing fees add to epoch #${currentEpochIndex}. ${storageFees} storage fees add to distribution pool`);

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
    if (coreChainLock && coreChainLock.height > header.coreChainLockedHeight) {
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
        appVersion: header.version.app,
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

    return new ResponseEndBlock({
      consensusParamUpdates,
      validatorSetUpdate,
      nextCoreChainLockUpdate,
    });
  }

  return endBlockHandler;
}

module.exports = endBlockHandlerFactory;
