const determineStatus = require("../determineStatus");

module.exports = async (coreService, dockerCompose, config) => {
  let [mnsyncStatus, networkInfo, blockchainInfo, peerInfo, status] = await Promise.all([
    coreService.getRpcClient().mnsync('status'),
    coreService.getRpcClient().getNetworkInfo(),
    coreService.getRpcClient().getBlockchainInfo(),
    coreService.getRpcClient().getPeerInfo(),
    determineStatus(dockerCompose, config, 'core')
  ])

  const {
    result: {
      AssetName: syncAsset,
      IsSynced: isSynced,
    },
  } = mnsyncStatus;

  const {subversion} = networkInfo.result;
  const coreVersion = subversion.replace(/\/|\(.*?\)|Dash Core:/g, '').replace(/\/|\(.*?\)/g, '');

  const {blocks, headers, chain, verificationprogress, difficulty} = blockchainInfo.result

  const {result: peers} = peerInfo

  if (status === 'running' && syncAsset !== 'MASTERNODE_SYNC_FINISHED') {
    status = `syncing`;
  }

  return {
    version: coreVersion,
    verificationProgress: verificationprogress.toFixed(2),
    blockHeight: blocks,
    headerHeight: headers,
    network: chain,
    isSynced,
    syncAsset,
    status,
    difficulty,
    peersCount: peers.length,
  }
}
