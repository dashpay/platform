const fs = require('node:fs');
const path = require('node:path');

const {
  v0: {
    GetStatusRequest,
    GetStatusResponse,
  },
} = require('@dashevo/dapi-grpc');
const BlockchainListener = require('../../../externalApis/tenderdash/BlockchainListener');

/**
 * @param {BlockchainListener} blockchainListener
 * @param {PlatformPromiseClient} driveClient
 * @param {jaysonClient} tenderdashRpcClient
 * @return {getStatusHandler}
 */
function getStatusHandlerFactory(blockchainListener, driveClient, tenderdashRpcClient) {
  let cachedResponse = null;

  blockchainListener.on(BlockchainListener.EVENTS.NEW_BLOCK, () => {
    cachedResponse = null;
  });

  const dapiSoftwareVersion = fs.readFileSync(path.join(__dirname, '../../../../package.json'), 'utf8');

  /**
   * @typedef {Function} getStatusHandler
   * @param {Object} call
   * @return {Promise<GetStatusResponse>}
   */
  function getStatusHandler() {
    if (cachedResponse) {
      return cachedResponse;
    }

    const request = new GetStatusRequest();

    const promises = [
      driveClient.getStatus(request),
      tenderdashRpcClient.request('status'),
      tenderdashRpcClient.request('net_info'),
    ];

    const [
      driveStatus,
      tenderdashStatusResponse,
      tenderdashNetInfoResponse,
    ] = Promise.allSettled(promises)
      .then((results) => {
        return results.map((result) => {
          if (result.status === 'fulfilled') {
            return result.value;
          }

          return {};
        });
      });

    let tenderdashStatus = {};
    if (tenderdashStatusResponse.result) {
      tenderdashStatus = tenderdashStatusResponse.result;
    }

    let tenderdashNetInfo = {};
    if (tenderdashNetInfoResponse.result) {
      tenderdashNetInfo = tenderdashNetInfoResponse.result
    }

    const v0 = new GetStatusResponse
      .GetStatusResponseV0();

    const version = new GetStatusResponse
      .GetStatusResponseV0.Version();

    // Versions

    const versionProtocolTenderdash = new GetStatusResponse
      .GetStatusResponseV0.Version.Protocol.Tenderdash();

    if (tenderdashStatus.node_info?.protocol_version) {
      versionProtocolTenderdash.setBlock(
        parseInt(tenderdashStatus.node_info.protocol_version.block),
      );

      versionProtocolTenderdash.setP2p(
        parseInt(tenderdashStatus.node_info.protocol_version.p2p),
      );
    }

    const versionProtocolDrive = new GetStatusResponse
      .GetStatusResponseV0.Version.Protocol.Drive();

    versionProtocolDrive.setCurrent(driveStatus.getVersion().getProtocol().getDrive().getCurrent());
    versionProtocolDrive.setMax(driveStatus.getVersion().getProtocol().getDrive().getMax());

    const versionProtocol = new GetStatusResponse
      .GetStatusResponseV0.Version.Protocol();

    versionProtocol.setTenderdash(versionProtocolTenderdash);
    versionProtocol.setDrive(versionProtocolDrive);

    version.setProtocol(versionProtocol);

    const versionSoftware = new GetStatusResponse
      .GetStatusResponseV0.Version.Software();

    versionSoftware.setDapi(dapiSoftwareVersion);
    if (driveStatus.getVersion()?.getSoftware()?.getDrive()) {
      versionSoftware.setDrive(driveStatus.getVersion()
        ?.getSoftware()
        ?.getDrive());
    }
    if (tenderdashStatus.node_info?.version) {
      versionSoftware.setTenderdash(tenderdashStatus.node_info?.version);
    }

    version.setSoftware(versionSoftware);

    v0.setVersion(version);

    // Node

    if (tenderdashStatus.node_info) {
      const node = new GetStatusResponse
        .GetStatusResponseV0.Node();

      node.setId(Buffer.from(tenderdashStatus.node_info.id, 'hex'));

      // ProTxHash is optional. This is present only for masternodes
      if (tenderdashStatus.node_info.ProTxHash) {
        node.setProTxHash(Buffer.from(tenderdashStatus.node_info.ProTxHash, 'hex'));
      }

      v0.setNode(node);
    }

    // Chain
    if (tenderdashStatus.sync_info) {
      const chain = new GetStatusResponse.GetStatusResponseV0.Chain();

      chain.setCatchingUp(tenderdashStatus.sync_info.catching_up);
      chain.setLatestBlockHash(Buffer.from(tenderdashStatus.sync_info.latest_block_hash, 'hex'));
      chain.setLatestAppHash(Buffer.from(tenderdashStatus.sync_info.latest_app_hash, 'hex'));
      chain.setLatestBlockHeight(Number(tenderdashStatus.sync_info.latest_block_height));
      chain.setEarliestBlockHash(Buffer.from(tenderdashStatus.sync_info.earliest_block_hash, 'hex'));
      chain.setEarliestAppHash(Buffer.from(tenderdashStatus.sync_info.earliest_app_hash, 'hex'));
      chain.setEarliestBlockHeight(Number(tenderdashStatus.sync_info.earliest_block_height));
      chain.setMaxPeerBlockHeight(Number(tenderdashStatus.sync_info.max_peer_block_height));
      chain.setCoreChainLockedHeight(driveStatus.getChain()?.getCoreChainLockedHeight());

      v0.setChain(chain);

      const stateSync = new GetStatusResponse.GetStatusResponseV0.StateSync();
      stateSync.setTotalSyncedTime(Number(tenderdashStatus.sync_info.total_synced_time));
      stateSync.setRemainingTime(value: number);
      stateSync.setTotalSnapshots(value: number);
      stateSync.setChunkProcessAvgTime(value: number);
      stateSync.setSnapshotHeight(value: number);
      stateSync.setSnapshotChunksCount(value: number);
      stateSync.setBackfilledBlocks(value: number);
      stateSync.setBackfillBlocksTotal(value: number);
    }

    // Network
    if (tenderdashNetInfo) {
      const network = new GetStatusResponse.GetStatusResponseV0.Network();

      network.setListening(tenderdashNetInfo.listening);

      if (tenderdashStatus.node_info) {
        network.setChainId(tenderdashStatus.node_info.network);
      }

      network.setPeersCount(Number(tenderdashNetInfo.n_peers));

      v0.setNetwork(network);
    }

    cachedResponse = new GetStatusResponse();
    cachedResponse.setVersion(v0);

    return cachedResponse;
  }

  return getStatusHandler;
}

module.exports = getStatusHandlerFactory;
