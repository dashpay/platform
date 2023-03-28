/**
 * @returns {getOverviewScopeFactory}
 * @param getCoreScope {getCoreScope}
 * @param getMasternodeScope {getMasternodeScope}
 * @param getPlatformScope {getPlatformScope}
 */
function getOverviewScopeFactory(getCoreScope,
  getMasternodeScope,
  getPlatformScope) {
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
      sentinel: {
        version: null,
        state: null,
      },
    };

    const platform = {
      enabled: config.isPlatformEnabled(),
      tenderdash: null,
    };

    if (masternodeEnabled) {
      const {
        state, proTxHash, sentinel, nodeState,
      } = await getMasternodeScope(config);

      masternode.state = state;
      masternode.proTxHash = proTxHash;
      masternode.sentinel = sentinel;
      masternode.nodeState = nodeState;
    }

    if (config.isPlatformEnabled()) {
      const { tenderdash } = await getPlatformScope(config);

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

module.exports = getOverviewScopeFactory;
