const getPaymentQueuePosition = require('../../util/getPaymentQueuePosition');
const blocksToTime = require('../../util/blocksToTime');

const getMasternodeScope = require('./masternode');
const getPlatformScope = require('./platform');
const extractCoreVersion = require('../../util/extractCoreVersion');
const determineStatus = require('../determineStatus');

module.exports = async (coreService, dockerCompose, config) => {
  const [blockchainInfo, networkInfo, status] = await Promise.all([
    coreService.getRpcClient().getBlockchainInfo(),
    coreService.getRpcClient().getNetworkInfo(),
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
    const { masternode: masternodeStatus, state } = await getMasternodeScope(config, ['masternode']);

    masternode.status = masternodeStatus;
    masternode.state = state;
  }

  if (platformEnabled) {
    const platformScope = await getPlatformScope(coreService, dockerCompose, config);

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
