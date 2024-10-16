/**
 * @returns {getOverviewScopeFactory}
 * @param getCoreScope {getCoreScope}
 * @param getMasternodeScope {getMasternodeScope}
 * @param getPlatformScope {getPlatformScope}
 */
export default function getOverviewScopeFactory(
  getCoreScope,
  getMasternodeScope,
  getPlatformScope,
) {
  /**
   * Get overview status scope
   *
   * @typedef {Promise} getOverviewScope
   * @param {Config} config
   * @returns {Promise<Object>}
   */
  async function getOverviewScope(config) {
    const masternodeEnabled = config.get('core.masternode.enable');

    const {
      serviceStatus, dockerStatus, verificationProgress,
      blockHeight, sizeOnDisk, version, network,
    } = await getCoreScope(config);

    const core = {
      network,
      version,
      dockerStatus,
      serviceStatus,
      verificationProgress,
      blockHeight,
      sizeOnDisk,
    };

    const masternode = {
      enabled: masternodeEnabled,
      proTxHash: null,
      nodeState: null,
      state: null,
    };

    const platform = {
      enabled: config.get('platform.enable'),
      tenderdash: null,
    };

    if (masternodeEnabled) {
      const {
        state, proTxHash, nodeState,
      } = await getMasternodeScope(config);

      masternode.state = state;
      masternode.proTxHash = proTxHash;
      masternode.nodeState = nodeState;
    }

    if (config.get('platform.enable')) {
      const { drive, tenderdash } = await getPlatformScope(config);

      platform.drive = drive;
      platform.tenderdash = tenderdash;
    }

    return {
      network,
      core,
      platform,
      masternode,
    };
  }

  return getOverviewScope;
}
