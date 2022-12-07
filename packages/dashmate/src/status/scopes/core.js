const determineStatus = require('../determineStatus');
const providers = require('../providers');
const extractCoreVersion = require('../../util/extractCoreVersion');

module.exports = async (createRpcClient, dockerCompose, config) => {
  const rpcClient = createRpcClient({
    port: config.get('core.rpc.port'),
    user: config.get('core.rpc.user'),
    pass: config.get('core.rpc.password'),
  })

  const [mnsyncStatus, networkInfo, blockchainInfo, peerInfo, dockerStatus] = await Promise.all([
    rpcClient.mnsync('status'),
    rpcClient.getNetworkInfo(),
    rpcClient.getBlockchainInfo(),
    rpcClient.getPeerInfo(),
    determineStatus.docker(dockerCompose, config, 'core'),
  ]);

  const [latestVersion, p2pPortState, remoteBlockHeight] = await Promise.all([
    providers.github.release('dashpay/dash'),
    providers.mnowatch.checkPortStatus(config.get('core.p2p.port')),
    providers.insight(config.get('network')).status(),
  ]);

  const network = config.get('network');
  const masternodeEnabled = config.get('core.masternode.enable');
  const rpcService = `127.0.0.1:${config.get('core.rpc.port')}`;
  const p2pService = `${config.get('externalIp')}:${config.get('core.p2p.port')}`;

  const blockHeight = blockchainInfo.result.blocks;
  const headerHeight = blockchainInfo.result.headers;
  const verificationProgress = blockchainInfo.result.verificationprogress.toFixed(4);
  const {chain, difficulty} = blockchainInfo.result;

  const peersCount = peerInfo.result.length;

  const {subversion} = networkInfo.result;
  const version = extractCoreVersion(subversion);

  const {AssetName: syncAsset} = mnsyncStatus.result;

  const serviceStatus = determineStatus.core(dockerStatus, syncAsset)

  const masternode = {
    enabled: masternodeEnabled,
  };

  return {
    version,
    network,
    chain,
    latestVersion,
    dockerStatus,
    serviceStatus,
    syncAsset,
    peersCount,
    p2pService,
    p2pPortState,
    rpcService,
    blockHeight,
    remoteBlockHeight,
    headerHeight,
    difficulty,
    verificationProgress,
    masternode,
  };
};
