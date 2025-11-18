const VersionStatus = require('./VersionStatus');
const NodeStatus = require('./NodeStatus');
const ChainStatus = require('./ChainStatus');
const TimeStatus = require('./TimeStatus');
const StateSyncStatus = require('./StateSyncStatus');
const NetworkStatus = require('./NetworkStatus');

class GetStatusResponse {
  /**
   * @param {VersionStatus} version - status versions
   * @param {NodeStatus} node - node status
   * @param {ChainStatus} chain - chain status
   * @param {NetworkStatus} network - network status
   * @param {StateSyncStatus} stateSync - state sync status
   * @param {TimeStatus} time - time status
   */
  constructor(version, node, chain, network, stateSync, time) {
    this.version = version;
    this.node = node;
    this.chain = chain;
    this.network = network;
    this.stateSync = stateSync;
    this.time = time;
  }

  /**
   * @returns {VersionStatus} network versions status
   */
  getVersionStatus() {
    return this.version;
  }

  /**
   * @returns {NodeStatus} node info status
   */
  getNodeStatus() {
    return this.node;
  }

  /**
   * @returns {ChainStatus} chain status
   */
  getChainStatus() {
    return this.chain;
  }

  /**
   * @returns {NetworkStatus} network status
   */
  getNetworkStatus() {
    return this.network;
  }

  /**
   * @returns {StateSyncStatus} state sync status
   */
  getStateSyncStatus() {
    return this.stateSync;
  }

  /**
   * @returns {TimeStatus} time status
   */
  getTimeStatus() {
    return this.time;
  }

  /**
   * @param {GetStatusResponse} proto GRPC GetStatusResponse
   * @returns {GetStatusResponse} JS DAPI Client GetStatusResponse
   */
  static createFromProto(proto) {
    const v0 = proto.getV0();

    const dapiVersion = v0.getVersion().getSoftware().getDapi();
    const driveVersion = v0.getVersion().getSoftware().getDrive();
    const tenderdashVersion = v0.getVersion().getSoftware().getTenderdash();
    const tenderdashP2pProtocol = v0.getVersion().getProtocol().getTenderdash().getP2p();
    const tenderdashBlockProtocol = v0.getVersion().getProtocol().getTenderdash().getBlock();
    const driveCurrentProtocol = v0.getVersion().getProtocol().getDrive().getCurrent();
    const driveLatestProtocol = v0.getVersion().getProtocol().getDrive().getLatest();
    const driveNextEpochProtocol = v0.getVersion().getProtocol().getDrive().getNextEpoch();

    const version = new VersionStatus(
      dapiVersion,
      driveVersion,
      tenderdashVersion,
      tenderdashP2pProtocol,
      tenderdashBlockProtocol,
      driveCurrentProtocol,
      driveLatestProtocol,
      driveNextEpochProtocol,
    );

    const nodeId = Buffer.from(v0.getNode().getId()).toString('hex');
    const proTxHash = Buffer.from(v0.getNode().getProTxHash()).toString('hex');

    const node = new NodeStatus(nodeId, proTxHash);

    const catchingUp = v0.getChain().getCatchingUp();
    const latestBlockHash = Buffer.from(v0.getChain().getLatestBlockHash()).toString('hex');
    const latestAppHash = Buffer.from(v0.getChain().getLatestAppHash()).toString('hex');
    const latestBlockHeight = BigInt(v0.getChain().getLatestBlockHeight());
    const earliestBlockHash = Buffer.from(v0.getChain().getEarliestBlockHash()).toString('hex');
    const earliestAppHash = Buffer.from(v0.getChain().getEarliestAppHash()).toString('hex');
    const earliestBlockHeight = BigInt(v0.getChain().getEarliestBlockHeight());
    const maxPeerBlockHeight = BigInt(v0.getChain().getMaxPeerBlockHeight());
    const coreChainLockedHeight = v0.getChain().getCoreChainLockedHeight();

    const chain = new ChainStatus(
      catchingUp,
      latestBlockHash,
      latestAppHash,
      latestBlockHeight,
      earliestBlockHash,
      earliestAppHash,
      earliestBlockHeight,
      maxPeerBlockHeight,
      coreChainLockedHeight,
    );

    const chainId = v0.getNetwork().getChainId();
    const peersCount = v0.getNetwork().getPeersCount();
    const isListening = v0.getNetwork().getListening();

    const network = new NetworkStatus(chainId, peersCount, isListening);

    const totalSyncedTime = BigInt(v0.getStateSync().getTotalSyncedTime());
    const remainingTime = BigInt(v0.getStateSync().getRemainingTime());
    const totalSnapshots = v0.getStateSync().getTotalSnapshots();
    const chunkProcessAverageTime = BigInt(v0.getStateSync().getChunkProcessAvgTime());
    const snapshotHeight = BigInt(v0.getStateSync().getSnapshotHeight());
    const snapshotChunksCount = BigInt(v0.getStateSync().getSnapshotChunksCount());
    const backfilledBlocks = BigInt(v0.getStateSync().getBackfilledBlocks());
    const backfillBlocksTotal = BigInt(v0.getStateSync().getBackfillBlocksTotal());

    const stateSync = new StateSyncStatus(
      totalSyncedTime,
      remainingTime,
      totalSnapshots,
      chunkProcessAverageTime,
      snapshotHeight,
      snapshotChunksCount,
      backfilledBlocks,
      backfillBlocksTotal,
    );

    const local = BigInt(v0.getTime().getLocal());
    const block = BigInt(v0.getTime().getBlock());
    const genesis = BigInt(v0.getTime().getGenesis());
    const epoch = v0.getTime().getEpoch();

    const time = new TimeStatus(local, block, genesis, epoch);

    return new GetStatusResponse(
      version,
      node,
      chain,
      network,
      stateSync,
      time,
    );
  }
}

module.exports = GetStatusResponse;
