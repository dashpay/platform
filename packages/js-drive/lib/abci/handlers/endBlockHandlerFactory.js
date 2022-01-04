const {
  tendermint: {
    abci: {
      ResponseEndBlock,
      ConsensusParams,
    },
    types: {
      CoreChainLock,
    },
  },
} = require('@dashevo/abci/types');

const featureFlagTypes = require('@dashevo/feature-flags-contract/lib/featureFlagTypes');

const NoSystemContractFoundError = require('./errors/NoSystemContractFoundError');

/**
 * Begin block ABCI handler
 *
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {number} dpnsContractBlockHeight
 * @param {Identifier} dpnsContractId
 * @param {number} dashpayContractBlockHeight
 * @param {Identifier} dashpayContractId
 * @param {LatestCoreChainLock} latestCoreChainLock
 * @param {ValidatorSet} validatorSet
 * @param {createValidatorSetUpdate} createValidatorSetUpdate
 * @param {BaseLogger} logger
 * @param {getFeatureFlagForHeight} getFeatureFlagForHeight
 * @param {BlockExecutionStoreTransactions} blockExecutionStoreTransactions
 * @param {Identifier} featureFlagDataContractId
 * @param {Long} featureFlagDataContractBlockHeight
 * @param {Identifier} masternodeRewardSharesContractId
 * @param {Long} masternodeRewardSharesContractBlockHeight
 * @return {endBlockHandler}
 */
function endBlockHandlerFactory(
  blockExecutionContext,
  dpnsContractBlockHeight,
  dpnsContractId,
  dashpayContractBlockHeight,
  dashpayContractId,
  latestCoreChainLock,
  validatorSet,
  createValidatorSetUpdate,
  logger,
  getFeatureFlagForHeight,
  blockExecutionStoreTransactions,
  featureFlagDataContractId,
  featureFlagDataContractBlockHeight,
  masternodeRewardSharesContractId,
  masternodeRewardSharesContractBlockHeight,
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

    if (dpnsContractId && height.equals(dpnsContractBlockHeight)) {
      if (!blockExecutionContext.hasDataContract(dpnsContractId)) {
        throw new NoSystemContractFoundError(
          'DPNS',
          dpnsContractId,
          dpnsContractBlockHeight,
        );
      }
    }

    if (dashpayContractId && height.equals(dashpayContractBlockHeight)) {
      if (!blockExecutionContext.hasDataContract(dashpayContractId)) {
        throw new NoSystemContractFoundError(
          'Dashpay',
          dashpayContractId,
          dashpayContractBlockHeight,
        );
      }
    }

    if (featureFlagDataContractId && height.equals(featureFlagDataContractBlockHeight)) {
      if (!blockExecutionContext.hasDataContract(featureFlagDataContractId)) {
        throw new NoSystemContractFoundError(
          'Feature flags',
          featureFlagDataContractId,
          featureFlagDataContractBlockHeight.toNumber(),
        );
      }
    }

    if (masternodeRewardSharesContractId
      && height.equals(masternodeRewardSharesContractBlockHeight)) {
      if (!blockExecutionContext.hasDataContract(masternodeRewardSharesContractId)) {
        throw new NoSystemContractFoundError(
          'Masternode reward shares',
          masternodeRewardSharesContractId,
          masternodeRewardSharesContractBlockHeight.toNumber(),
        );
      }
    }

    const header = blockExecutionContext.getHeader();
    const lastCommitInfo = blockExecutionContext.getLastCommitInfo();
    const coreChainLock = latestCoreChainLock.getChainLock();

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
    const documentsTransaction = blockExecutionStoreTransactions.getTransaction('documents');

    const updateConsensusParamsFeatureFlag = await getFeatureFlagForHeight(
      featureFlagTypes.UPDATE_CONSENSUS_PARAMS, height, documentsTransaction,
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
