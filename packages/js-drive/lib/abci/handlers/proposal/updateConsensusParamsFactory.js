const {
  tendermint: {
    types: {
      ConsensusParams,
    },
  },
} = require('@dashevo/abci/types');

const featureFlagTypes = require('@dashevo/feature-flags-contract/lib/featureFlagTypes');

/**
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {getFeatureFlagForHeight} getFeatureFlagForHeight
 * @return {updateConsensusParams}
 */
function updateConsensusParamsFactory(
  blockExecutionContext,
  getFeatureFlagForHeight,
) {
  /**
   * @typedef updateConsensusParams
   * @param {number} height
   * @param {BaseLogger} logger
   * @return {Promise<ConsensusParams>}
   */
  async function updateConsensusParams(height, logger) {
    const consensusLogger = logger.child({
      height: height.toString(),
      abciMethod: 'updateConsensusParams',
    });

    const contextVersion = blockExecutionContext.getVersion();

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

    return consensusParamUpdates;
  }

  return updateConsensusParams;
}

module.exports = updateConsensusParamsFactory;
