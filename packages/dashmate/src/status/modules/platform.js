const fetch = require('node-fetch')
const determineStatus = require("../determineStatus");

module.exports = async (coreService, dockerCompose, config) => {
  const {
    result: {
      IsSynced: coreIsSynced,
    },
  } = await coreService.getRpcClient().mnsync('status');

  const status = await determineStatus(dockerCompose, config, 'drive_tenderdash')

  const tenderdash = {
    version: null,
    catchingUp: null,
    lastBlockHeight: null,
    latestAppHash: null,
    peers: null,
    network: null,
  }

  if (coreIsSynced) {
    // Collecting platform data fails if Tenderdash is waiting for core to sync
    try {
      const tenderdashStatus = await fetch(`http://localhost:${config.get('platform.drive.tenderdash.rpc.port')}/status`);

      const {node_info, sync_info} = await tenderdashStatus.json()
      const {version, network} = node_info
      const {catching_up, latest_block_height, latest_app_hash} = sync_info

      const tenderdashNetInfoRes = await fetch(`http://localhost:${config.get('platform.drive.tenderdash.rpc.port')}/net_info`);
      const {
        n_peers: platformPeers,
      } = await tenderdashNetInfoRes.json();

      tenderdash.version = version
      tenderdash.lastBlockHeight = latest_block_height
      tenderdash.catchingUp = catching_up
      tenderdash.peers = platformPeers
      tenderdash.network = network
      tenderdash.latestAppHash = latest_app_hash
    } catch (e) {
      if (e.name !== 'FetchError') {
        throw e;
      }
    }
  }

  return {
    status,
    coreIsSynced,
    tenderdash
  }
}
