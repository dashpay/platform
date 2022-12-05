const getMasternodeScope = require('./masternode');
const getPlatformScope = require('./platform');
const extractCoreVersion = require('../../util/extractCoreVersion');
const determineStatus = require('../determineStatus');
const createRpcClient = require("../../core/createRpcClient");

module.exports = async (dockerCompose, config) => {
  const rpcClient = createRpcClient({
    port: config.get('core.rpc.port'),
    user: config.get('core.rpc.user'),
    pass: config.get('core.rpc.password'),
  })

  const [blockchainInfo, networkInfo, status] = await Promise.all([
    rpcClient.getBlockchainInfo(),
    rpcClient.getNetworkInfo(),
    determineStatus(dockerCompose, config, 'core'),
  ]);

  const network = config.get('network');
  const masternodeEnabled = config.get('core.masternode.enable');
  const platformEnabled = config.get('network') !== 'mainnet' && config.name !== 'local_seed';

  const sizeOnDisk = blockchainInfo.result.size_on_disk;
  const blockHeight = blockchainInfo.result.blocks;
  const verificationProgress = blockchainInfo.result.verificationprogress.toFixed(4);

  const { subversion } = networkInfo.result;
  const version = extractCoreVersion(subversion);

  const core = {
    version,
    status,
    verificationProgress,
    blockHeight,
    sizeOnDisk,
  };

  const masternode = {
    enabled: masternodeEnabled,
    status,
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
    status: null,
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
    const { masternode: masternodeStatus, state } = await getMasternodeScope(dockerCompose, config);

    masternode.status = masternodeStatus;
    masternode.state = state;
  }

  if (platformEnabled) {
    const platformScope = await getPlatformScope(dockerCompose, config);

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
