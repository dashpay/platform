const VersionStatus = require('./VersionStatus');
const NodeStatus = require('./NodeStatus');
const ChainStatus = require('./ChainStatus');
const TimeStatus = require('./TimeStatus');
const StateSyncStatus = require('./StateSyncStatus');
const NetworkStatus = require('./NetworkStatus');

/**
 * Read a value out of an optional protobuf sub-message.
 *
 * Every message-typed field in `GetStatusResponseV0` is optional on the wire, and DAPI
 * omits whole sections when the underlying data is unavailable: `state_sync` is absent
 * unless the node is state syncing, `chain`/`node`/`network` are absent when Tenderdash
 * is unreachable, and `protocol.drive` is absent when Drive is unreachable. The generated
 * getters return `undefined` for an absent sub-message, so they must never be chained
 * without a guard.
 *
 * @param {object|undefined} message - protobuf sub-message, possibly absent
 * @param {function(object): *} read - reader invoked when the sub-message is present
 * @returns {*} the read value, or `undefined` when the sub-message is absent
 */
function readOptional(message, read) {
  return message ? read(message) : undefined;
}

/**
 * Convert an optional numeric protobuf field to BigInt, preserving absence.
 *
 * @param {string|number|undefined} value
 * @returns {bigint|undefined}
 */
function toOptionalBigInt(value) {
  return value === undefined || value === null ? undefined : BigInt(value);
}

class GetStatusResponse {
  /**
   * @param {VersionStatus} version - status versions
   * @param {NodeStatus|null} node - node status, null if unavailable
   * @param {ChainStatus|null} chain - chain status, null if unavailable
   * @param {NetworkStatus|null} network - network status, null if unavailable
   * @param {StateSyncStatus|null} stateSync - state sync status, null if not state syncing
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
   * @returns {NodeStatus|null} node info status, null if unavailable
   */
  getNodeStatus() {
    return this.node;
  }

  /**
   * @returns {ChainStatus|null} chain status, null if unavailable
   */
  getChainStatus() {
    return this.chain;
  }

  /**
   * @returns {NetworkStatus|null} network status, null if unavailable
   */
  getNetworkStatus() {
    return this.network;
  }

  /**
   * @returns {StateSyncStatus|null} state sync status, null if not state syncing
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

    const versionProto = v0.getVersion();
    const softwareProto = readOptional(versionProto, (v) => v.getSoftware());
    const protocolProto = readOptional(versionProto, (v) => v.getProtocol());
    const tenderdashProtocolProto = readOptional(protocolProto, (p) => p.getTenderdash());
    const driveProtocolProto = readOptional(protocolProto, (p) => p.getDrive());

    const version = new VersionStatus(
      readOptional(softwareProto, (s) => s.getDapi()),
      readOptional(softwareProto, (s) => s.getDrive()),
      readOptional(softwareProto, (s) => s.getTenderdash()),
      readOptional(tenderdashProtocolProto, (t) => t.getP2p()),
      readOptional(tenderdashProtocolProto, (t) => t.getBlock()),
      readOptional(driveProtocolProto, (d) => d.getCurrent()),
      readOptional(driveProtocolProto, (d) => d.getLatest()),
      readOptional(driveProtocolProto, (d) => d.getNextEpoch()),
    );

    const nodeProto = v0.getNode();

    const node = nodeProto ? new NodeStatus(
      Buffer.from(nodeProto.getId()).toString('hex'),
      Buffer.from(nodeProto.getProTxHash()).toString('hex'),
    ) : null;

    const chainProto = v0.getChain();

    const chain = chainProto ? new ChainStatus(
      chainProto.getCatchingUp(),
      Buffer.from(chainProto.getLatestBlockHash()).toString('hex'),
      Buffer.from(chainProto.getLatestAppHash()).toString('hex'),
      BigInt(chainProto.getLatestBlockHeight()),
      Buffer.from(chainProto.getEarliestBlockHash()).toString('hex'),
      Buffer.from(chainProto.getEarliestAppHash()).toString('hex'),
      BigInt(chainProto.getEarliestBlockHeight()),
      BigInt(chainProto.getMaxPeerBlockHeight()),
      chainProto.getCoreChainLockedHeight(),
    ) : null;

    const networkProto = v0.getNetwork();

    const network = networkProto ? new NetworkStatus(
      networkProto.getChainId(),
      networkProto.getPeersCount(),
      networkProto.getListening(),
    ) : null;

    // DAPI omits the whole state sync section on nodes that are not state syncing,
    // so an absent section means "no state sync information", not "all zeroes".
    const stateSyncProto = v0.getStateSync();

    const stateSync = stateSyncProto ? new StateSyncStatus(
      BigInt(stateSyncProto.getTotalSyncedTime()),
      BigInt(stateSyncProto.getRemainingTime()),
      stateSyncProto.getTotalSnapshots(),
      BigInt(stateSyncProto.getChunkProcessAvgTime()),
      BigInt(stateSyncProto.getSnapshotHeight()),
      BigInt(stateSyncProto.getSnapshotChunksCount()),
      BigInt(stateSyncProto.getBackfilledBlocks()),
      BigInt(stateSyncProto.getBackfillBlocksTotal()),
    ) : null;

    const timeProto = v0.getTime();

    const time = new TimeStatus(
      toOptionalBigInt(readOptional(timeProto, (t) => t.getLocal())),
      toOptionalBigInt(readOptional(timeProto, (t) => t.getBlock())),
      toOptionalBigInt(readOptional(timeProto, (t) => t.getGenesis())),
      readOptional(timeProto, (t) => t.getEpoch()),
    );

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
