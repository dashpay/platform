const determineStatus = require("../determineStatus");
const providers = require("../providers")
const extractCoreVersion = require("../../util/extractCoreVersion");
const ServiceStatusEnum = require("../../enums/serviceStatus");

module.exports = async (coreService, dockerCompose, config) => {
  const [mnsyncStatus, networkInfo, blockchainInfo, peerInfo, status] = await Promise.all([
    coreService.getRpcClient().mnsync('status'),
    coreService.getRpcClient().getNetworkInfo(),
    coreService.getRpcClient().getBlockchainInfo(),
    coreService.getRpcClient().getPeerInfo(),
    determineStatus(dockerCompose, config, 'core')
  ])

  let [latestVersion, p2pPortState, remoteBlockHeight] = await Promise.all([
    providers.github.release('dashpay/dash'),
    providers.mnowatch.checkPortStatus(config.get('core.p2p.port')),
    providers.insight(config.get('network')).status()
  ])

  const network = config.get('network')
  const masternodeEnabled = config.get('core.masternode.enable')
  const rpcService = `127.0.0.1:${config.get('core.rpc.port')}`
  const p2pService = `${config.get('externalIp')}:${config.get('core.p2p.port')}`

  const blockHeight = blockchainInfo.result.blocks
  const headerHeight = blockchainInfo.result.headers
  const verificationProgress = blockchainInfo.result.verificationprogress.toFixed(4)
  const {chain, difficulty} = blockchainInfo.result

  const peersCount = peerInfo.result.length;

  const {subversion} = networkInfo.result;
  const version = extractCoreVersion(subversion)

  const {AssetName: syncAsset} = mnsyncStatus.result;

  const masternode = {
    enabled: masternodeEnabled,
    sentinel: {
      status: null,
      version: null,
    }
  }

  if (masternodeEnabled) {
    const {sentinelState, sentinelVersion} = masternode

    masternode.sentinel.status = sentinelState
    masternode.sentinel.version = sentinelVersion
  }

  return {
    version,
    network,
    chain,
    latestVersion,
    status,
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
    masternode
  }
}
