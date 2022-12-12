module.exports = (getCoreScope, getPlatformScope, getMasternodeScope) => async (config) => {
  const network = config.get('network');
  const masternodeEnabled = config.get('core.masternode.enable');
  const platformEnabled = config.get('network') !== 'mainnet' && config.name !== 'local_seed';

  const {
    serviceStatus, dockerStatus, verificationProgress,
    blockHeight, sizeOnDisk, version,
  } = await getCoreScope(config);

  const core = {
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
    enabled: platformEnabled,
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

  if (platformEnabled) {
    const { tenderdash } = await getPlatformScope(config);

    platform.tenderdash = tenderdash;
  }

  return {
    network,
    core,
    platform,
    masternode,
  };
};
