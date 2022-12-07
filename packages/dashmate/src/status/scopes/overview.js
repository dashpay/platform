const getMasternodeScope = require('./masternode');
const getPlatformScope = require('./platform');
const extractCoreVersion = require('../../util/extractCoreVersion');
const determineStatus = require('../determineStatus');

module.exports = async (createRpcClient, dockerCompose, config) => {
  const rpcClient = createRpcClient({
    port: config.get('core.rpc.port'),
    user: config.get('core.rpc.user'),
    pass: config.get('core.rpc.password'),
  })

  const [mnSync, blockchainInfo, networkInfo, dockerStatus] = await Promise.all([
    rpcClient.mnsync('status'),
    rpcClient.getBlockchainInfo(),
    rpcClient.getNetworkInfo(),
    determineStatus.docker(dockerCompose, config, 'core'),
  ]);

  const {AssetName: syncAsset} = mnSync.result;
  const serviceStatus = determineStatus.core(dockerStatus, syncAsset)

  const network = config.get('network');
  const masternodeEnabled = config.get('core.masternode.enable');
  const platformEnabled = config.get('network') !== 'mainnet' && config.name !== 'local_seed';

  const sizeOnDisk = blockchainInfo.result.size_on_disk;
  const blockHeight = blockchainInfo.result.blocks;
  const verificationProgress = blockchainInfo.result.verificationprogress.toFixed(4);

  const {subversion} = networkInfo.result;
  const version = extractCoreVersion(subversion);

  const core = {
    version,
    dockerStatus,
    serviceStatus:
    verificationProgress,
    blockHeight,
    sizeOnDisk,
  };

  const masternode = {
    enabled: masternodeEnabled,
    state: {
      poSePenalty: null,
      lastPaidHeight: null,
      lastPaidTime: null,
      paymentQueuePosition: null,
      nextPaymentTime: null,
    },
  };

  const platform = {
    enabled: platformEnabled,
    dockerStatus,
    serviceStatus,
    tenderdash: {
      version: null,
      lastBlockHeight: null,
      catchingUp: null,
      peers: null,
      network: null,
      latestAppHash: null,
    },
  };

  if (masternodeEnabled) {
    const masternodeScope = await getMasternodeScope(createRpcClient, dockerCompose, config);

    masternode.state = masternodeScope.state;
  }

  if (platformEnabled) {
    const platformScope = await getPlatformScope(createRpcClient, dockerCompose, config);

    platform.status = platformScope.status;
    platform.tenderdash = platformScope.tenderdash;
  }

  return {
    network,
    core,
    platform,
    masternode,
  };
};
