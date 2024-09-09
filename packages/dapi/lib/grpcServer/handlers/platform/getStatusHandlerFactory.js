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
  // Clean cache when new platform block committed
  let cachedResponse = null;

  blockchainListener.on(BlockchainListener.EVENTS.NEW_BLOCK, () => {
    cachedResponse = null;
  });

  // DAPI Software version
  const packageJsonPath = path.resolve(__dirname, '..', '..', '..', '..', 'package.json');
  const packageJsonString = fs.readFileSync(packageJsonPath, 'utf-8');
  const packageJson = JSON.parse(packageJsonString);
  const dapiSoftwareVersion = packageJson.version;

  /**
   * @typedef {Function} getStatusHandler
   * @return {Promise<GetStatusResponse>}
   */
  async function getStatusHandler() {
    // Return cached response if it exists
    if (cachedResponse !== null) {
      cachedResponse.getV0().getTime().setLocal(Date.now());

      return cachedResponse;
    }

    const request = new GetStatusRequest();

    const promises = [
      driveClient.getStatus(request)
        .then((response) => response.getV0()?.toObject() || {}),
      tenderdashRpcClient.request('status', {}),
      tenderdashRpcClient.request('net_info', {}),
    ];

    const [
      driveStatus,
      tenderdashStatusResponse,
      tenderdashNetInfoResponse,
    ] = await Promise.allSettled(promises)
      .then((results) => results.map((result) => {
        if (result.status === 'fulfilled') {
          return result.value;
        }

        return {};
      }));

    let tenderdashStatus = {};
    if (tenderdashStatusResponse.result) {
      tenderdashStatus = tenderdashStatusResponse.result;
    }

    let tenderdashNetInfo = {};
    if (tenderdashNetInfoResponse.result) {
      tenderdashNetInfo = tenderdashNetInfoResponse.result;
    }

    const v0 = new GetStatusResponse
      .GetStatusResponseV0();

    const version = new GetStatusResponse
      .GetStatusResponseV0.Version();

    // Versions

    const versionProtocol = new GetStatusResponse
      .GetStatusResponseV0.Version.Protocol();

    if (tenderdashStatus.node_info?.protocol_version) {
      const versionProtocolTenderdash = new GetStatusResponse
        .GetStatusResponseV0.Version.Protocol.Tenderdash();

      versionProtocolTenderdash.setBlock(
        Number(tenderdashStatus.node_info.protocol_version.block),
      );

      versionProtocolTenderdash.setP2p(
        Number(tenderdashStatus.node_info.protocol_version.p2p),
      );

      versionProtocol.setTenderdash(versionProtocolTenderdash);
    }

    if (driveStatus.version?.protocol?.drive) {
      const versionProtocolDrive = new GetStatusResponse
        .GetStatusResponseV0.Version.Protocol.Drive();

      versionProtocolDrive.setCurrent(driveStatus.version.protocol.drive.current);
      versionProtocolDrive.setLatest(driveStatus.version.protocol.drive.latest);

      versionProtocol.setDrive(versionProtocolDrive);
    }

    version.setProtocol(versionProtocol);

    const versionSoftware = new GetStatusResponse
      .GetStatusResponseV0.Version.Software();

    versionSoftware.setDapi(dapiSoftwareVersion);

    if (driveStatus.version?.software?.drive) {
      versionSoftware.setDrive(driveStatus.version.software.drive);
    }

    if (tenderdashStatus.node_info?.version) {
      versionSoftware.setTenderdash(tenderdashStatus.node_info.version);
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
      if (driveStatus.chain?.coreChainLockedHeight) {
        chain.setCoreChainLockedHeight(driveStatus.chain.coreChainLockedHeight);
      }

      v0.setChain(chain);

      const stateSync = new GetStatusResponse.GetStatusResponseV0.StateSync();
      stateSync.setTotalSyncedTime(Number(tenderdashStatus.sync_info.total_synced_time));
      stateSync.setRemainingTime(Number(tenderdashStatus.sync_info.remaining_time));
      stateSync.setTotalSnapshots(Number(tenderdashStatus.sync_info.total_snapshots));
      stateSync.setChunkProcessAvgTime(
        Number(tenderdashStatus.sync_info.chunk_process_avg_time),
      );
      stateSync.setSnapshotHeight(Number(tenderdashStatus.sync_info.snapshot_height));
      stateSync.setSnapshotChunksCount(Number(tenderdashStatus.sync_info.snapshot_chunks_count));
      stateSync.setBackfilledBlocks(Number(tenderdashStatus.sync_info.backfilled_blocks));
      stateSync.setBackfillBlocksTotal(Number(tenderdashStatus.sync_info.backfill_blocks_total));

      v0.setStateSync(stateSync);
    }

    // Network
    if (tenderdashNetInfo.listening !== undefined) {
      const network = new GetStatusResponse.GetStatusResponseV0.Network();

      network.setListening(tenderdashNetInfo.listening);

      if (tenderdashStatus.node_info) {
        network.setChainId(tenderdashStatus.node_info.network);
      }

      network.setPeersCount(Number(tenderdashNetInfo.n_peers));

      v0.setNetwork(network);
    }

    // Time

    const time = new GetStatusResponse.GetStatusResponseV0.Time();

    if (driveStatus.time) {
      time.setBlock(driveStatus.time.block);
      time.setGenesis(driveStatus.time.genesis);
      time.setEpoch(driveStatus.time.epoch);
    }

    time.setLocal(Date.now());

    v0.setTime(time);

    cachedResponse = new GetStatusResponse();
    cachedResponse.setV0(v0);

    return cachedResponse;
  }

  return getStatusHandler;
}

module.exports = getStatusHandlerFactory;
