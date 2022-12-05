const {
  tendermint: {
    types: {
      ConsensusParams,
    },
  },
} = require('@dashevo/abci/types');

const featureFlagTypes = require('@dashevo/feature-flags-contract/lib/featureFlagTypes');

/**
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 * @param {getFeatureFlagForHeight} getFeatureFlagForHeight
 * @return {createConsensusParamUpdate}
 */
function createConsensusParamUpdateFactory(
  proposalBlockExecutionContext,
  getFeatureFlagForHeight,
) {
  /**
   * @typedef createConsensusParamUpdate
   * @param {number} height
   * @param {number} round
   * @param {BaseLogger} consensusLogger
   * @return {Promise<ConsensusParams>}
   */
  async function createConsensusParamUpdate(height, round, consensusLogger) {
    const contextVersion = proposalBlockExecutionContext.getVersion();

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

  return createConsensusParamUpdate;
}

module.exports = createConsensusParamUpdateFactory;
