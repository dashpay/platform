const DockerStatusEnum = require('../../enums/dockerStatus');
const determineStatus = require('../determineStatus');
const providers = require('../providers');
const extractCoreVersion = require('../../util/extractCoreVersion');
const ServiceStatusEnum = require('../../enums/serviceStatus');

module.exports = (dockerCompose, createRpcClient) => async (config) => {
  const network = config.get('network');
  const rpcService = `127.0.0.1:${config.get('core.rpc.port')}`;
  const p2pService = `${config.get('externalIp')}:${config.get('core.p2p.port')}`;

  const rpcClient = createRpcClient({
    port: config.get('core.rpc.port'),
    user: config.get('core.rpc.user'),
    pass: config.get('core.rpc.password'),
  });

  const core = {
    network,
    p2pService,
    rpcService,
    version: null,
    chain: null,
    latestVersion: null,
    dockerStatus: null,
    serviceStatus: null,
    peersCount: null,
    p2pPortState: null,
    blockHeight: null,
    remoteBlockHeight: null,
    headerHeight: null,
    difficulty: null,
    verificationProgress: null,
  };

  core.dockerStatus = await determineStatus.docker(dockerCompose, config, 'core');

  if (core.dockerStatus !== DockerStatusEnum.running) {
    core.serviceStatus = ServiceStatusEnum.error;
    return core;
  }

  const [mnsyncStatus, networkInfo, blockchainInfo] = await Promise.all([
    rpcClient.mnsync('status'),
    rpcClient.getNetworkInfo(),
    rpcClient.getBlockchainInfo(),
  ]);

  const { AssetName: syncAsset } = mnsyncStatus.result;
  core.serviceStatus = determineStatus.core(core.dockerStatus, syncAsset);

  const {
    chain, difficulty, blocks, headers, verificationprogress,
  } = blockchainInfo.result;

  core.chain = chain;
  core.difficulty = difficulty;
  core.blockHeight = blocks;
  core.headerHeight = headers;
  core.verificationProgress = verificationprogress;

  const { subversion, connections } = networkInfo.result;

  core.peersCount = connections;
  core.version = extractCoreVersion(subversion);

  const providersResult = await Promise.allSettled([
    providers.github.release('dashpay/dash'),
    providers.mnowatch.checkPortStatus(config.get('core.p2p.port')),
    providers.insight(config.get('network')).status(),
  ]);

  const [latestVersion, p2pPortState, insightStatus] = providersResult
    .map((result) => (result.status === 'fulfilled' ? result.value : null));

  core.latestVersion = latestVersion;
  core.p2pPortState = p2pPortState;
  core.remoteBlockHeight = insightStatus ? insightStatus.info.blocks : null;

  return core;
};
