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

const { FEE_MULTIPLIER } = require('@dashevo/dpp/lib/stateTransition/fee/constants');

/**
 * Begin block ABCI handler
 *
 * @param {BlockExecutionContext} blockExecutionContext
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
        feeMultiplier: FEE_MULTIPLIER,
      },
    }, true);

    const { currentEpochIndex, isEpochChange } = rsResponse.epochInfo;

    if (isEpochChange) {
      const latestContext = blockExecutionContextStack.getLatest();

      if (latestContext) {
        const latestHeader = latestContext.getHeader();
        rsRequest.previousBlockTime = latestHeader.time.seconds + latestHeader.time.nanos;
      }

      consensusLogger.debug({
        currentEpochIndex,
        blockTime,
        previousBlocktime
      }, `Fee epoch #${currentEpochIndex} started on block time ${blockTime}`);
    }

    consensusLogger.debug({
      currentEpochIndex,
      masternodesPaidCount: rsResponse.masternodesPaidCount,
      feeMultiplier: FEE_MULTIPLIER,
      processingFees,
      storageFees,
    }, `${processingFees} processing fees add to epoch #${currentEpochIndex}. ${storageFees} storage fees add to distribution pool`);

    if (rsResponse.masternodesPaidCount) {
      consensusLogger.debug({
        currentEpochIndex,
        masternodesPaidCount: rsResponse.masternodesPaidCount,
      }, `${masternodesPaidCount} masternodes paid for epoch #${currentEpochIndex}`);
    }

    masternodesPaidCount

  * @property {EpochInfo} epochInfo
    * @property {number}
    */

    /**
     * @typedef EpochInfo
     * @property {number} currentEpochIndex
     * @property {boolean} isEpochChange
     */

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
