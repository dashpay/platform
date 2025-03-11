const {
  v0: {
    PlatformPromiseClient,
    GetStatusRequest,
    GetStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

const getStatusFixture = require('../../../../../lib/test/fixtures/getStatusFixture');
const getStatusFactory = require('../../../../../lib/methods/platform/getStatus/getStatusFactory');
const VersionStatus = require('../../../../../lib/methods/platform/getStatus/VersionStatus');
const NodeStatus = require('../../../../../lib/methods/platform/getStatus/NodeStatus');
const ChainStatus = require('../../../../../lib/methods/platform/getStatus/ChainStatus');
const NetworkStatus = require('../../../../../lib/methods/platform/getStatus/NetworkStatus');
const StateSyncStatus = require('../../../../../lib/methods/platform/getStatus/StateSyncStatus');
const TimeStatus = require('../../../../../lib/methods/platform/getStatus/TimeStatus');

describe('getStatusFactory', () => {
  let grpcTransportMock;
  let getStatus;
  let statusFixture;
  let response;
  let options;

  beforeEach(async function beforeEach() {
    statusFixture = getStatusFixture();

    const { GetStatusResponseV0 } = GetStatusResponse;
    response = new GetStatusResponse();

    response.setV0(
      new GetStatusResponseV0()
        .setVersion(new GetStatusResponseV0.Version()
          .setSoftware(
            new GetStatusResponse.GetStatusResponseV0.Version.Software()
              .setDapi(statusFixture.version.software.dapi)
              .setDrive(statusFixture.version.software.drive)
              .setTenderdash(statusFixture.version.software.tenderdash),
          )
          .setProtocol(
            new GetStatusResponse.GetStatusResponseV0.Version.Protocol()
              .setDrive(new GetStatusResponse.GetStatusResponseV0
                .Version.Protocol.Drive()
                .setLatest(statusFixture.version.protocol.drive.latest)
                .setCurrent(statusFixture.version.protocol.drive.current))
              .setTenderdash(new GetStatusResponse.GetStatusResponseV0
                .Version.Protocol.Tenderdash()
                .setP2p(statusFixture.version.protocol.tenderdash.p2p)
                .setBlock(statusFixture.version.protocol.tenderdash.block)),
          ))
        .setNode(new GetStatusResponse.GetStatusResponseV0.Node()
          .setId(statusFixture.node.id)
          .setProTxHash(statusFixture.node.proTxHash))
        .setChain(new GetStatusResponse.GetStatusResponseV0.Chain()
          .setCatchingUp(statusFixture.chain.catchingUp)
          .setLatestBlockHash(statusFixture.chain.latestBlockHash)
          .setLatestAppHash(statusFixture.chain.latestAppHash)
          .setLatestBlockHeight(statusFixture.chain.latestBlockHeight)
          .setEarliestBlockHash(statusFixture.chain.earliestBlockHash)
          .setEarliestAppHash(statusFixture.chain.earliestAppHash)
          .setEarliestBlockHeight(statusFixture.chain.earliestBlockHeight)
          .setMaxPeerBlockHeight(statusFixture.chain.maxPeerBlockHeight)
          .setCoreChainLockedHeight(statusFixture.chain.coreChainLockedHeight))
        .setNetwork(new GetStatusResponse.GetStatusResponseV0.Network()
          .setChainId(statusFixture.network.chainId)
          .setPeersCount(statusFixture.network.peersCount)
          .setListening(statusFixture.network.listening))
        .setStateSync(new GetStatusResponse.GetStatusResponseV0.StateSync()
          .setTotalSyncedTime(statusFixture.stateSync.totalSyncedTime)
          .setRemainingTime(statusFixture.stateSync.remainingTime)
          .setTotalSnapshots(statusFixture.stateSync.totalSnapshots)
          .setChunkProcessAvgTime(statusFixture.stateSync.chunkProcessAverageTime)
          .setSnapshotHeight(statusFixture.stateSync.snapshotHeight)
          .setSnapshotChunksCount(statusFixture.stateSync.snapshotChunksCount)
          .setBackfilledBlocks(statusFixture.stateSync.backfilledBlocks)
          .setBackfillBlocksTotal(statusFixture.stateSync.backfillBlocksTotal))
        .setTime(new GetStatusResponse.GetStatusResponseV0.Time()
          .setLocal(statusFixture.time.local)
          .setBlock(statusFixture.time.block)
          .setGenesis(statusFixture.time.genesis)
          .setEpoch(statusFixture.time.epoch)),
    );

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    getStatus = getStatusFactory(grpcTransportMock);
    options = {
      timeout: 1000,
    };
  });

  it('should return status from grpc', async () => {
    const result = await getStatus(options);

    const { GetStatusRequestV0 } = GetStatusRequest;
    const request = new GetStatusRequest();
    request.setV0(
      new GetStatusRequestV0(),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getStatus',
      request,
      options,
    );
    const versionStatus = result.getVersionStatus();
    const nodeStatus = result.getNodeStatus();
    const chainStatus = result.getChainStatus();
    const networkStatus = result.getNetworkStatus();
    const stateSyncStatus = result.getStateSyncStatus();
    const timeStatus = result.getTimeStatus();

    expect(versionStatus).to.be.an.instanceOf(VersionStatus);
    expect(versionStatus.getDriveVersion())
      .to.equal(statusFixture.version.software.drive);
    expect(versionStatus.getTenderdashVersion())
      .to.equal(statusFixture.version.software.tenderdash);
    expect(versionStatus.getTenderdashP2pProtocol())
      .to.equal(statusFixture.version.protocol.tenderdash.p2p);
    expect(versionStatus.getTenderdashBlockProtocol())
      .to.equal(statusFixture.version.protocol.tenderdash.block);
    expect(versionStatus.getDriveCurrentProtocol())
      .to.equal(statusFixture.version.protocol.drive.current);
    expect(versionStatus.getDriveLatestProtocol())
      .to.equal(statusFixture.version.protocol.drive.latest);

    expect(nodeStatus).to.be.an.instanceOf(NodeStatus);
    expect(nodeStatus.getNodeId()).to.equal(Buffer.from(statusFixture.node.id).toString('hex'));
    expect(nodeStatus.getProTxHash()).to.equal(Buffer.from(statusFixture.node.proTxHash).toString('hex'));

    expect(chainStatus).to.be.an.instanceOf(ChainStatus);
    expect(chainStatus.isCatchingUp())
      .to.equal(statusFixture.chain.catchingUp);
    expect(chainStatus.getLatestBlockHash())
      .to.equal(Buffer.from(statusFixture.chain.latestBlockHash).toString('hex'));
    expect(chainStatus.getLatestAppHash())
      .to.equal(Buffer.from(statusFixture.chain.latestAppHash).toString('hex'));
    expect(chainStatus.getLatestBlockHeight())
      .to.equal(BigInt(statusFixture.chain.latestBlockHeight));
    expect(chainStatus.getEarliestBlockHash())
      .to.equal(Buffer.from(statusFixture.chain.earliestBlockHash).toString('hex'));
    expect(chainStatus.getEarliestAppHash())
      .to.equal(Buffer.from(statusFixture.chain.earliestAppHash).toString('hex'));
    expect(chainStatus.getEarliestBlockHeight())
      .to.equal(BigInt(statusFixture.chain.earliestBlockHeight));
    expect(chainStatus.getMaxPeerBlockHeight())
      .to.equal(BigInt(statusFixture.chain.maxPeerBlockHeight));
    expect(chainStatus.getCoreChainLockedHeight())
      .to.equal(statusFixture.chain.coreChainLockedHeight);

    expect(networkStatus).to.be.an.instanceOf(NetworkStatus);
    expect(networkStatus.getChainId()).to.equal(statusFixture.network.chainId);
    expect(networkStatus.getPeersCount()).to.equal(statusFixture.network.peersCount);
    expect(networkStatus.isListening()).to.equal(statusFixture.network.listening);

    expect(stateSyncStatus).to.be.an.instanceOf(StateSyncStatus);
    expect(stateSyncStatus.getTotalSyncedTime())
      .to.equal(BigInt(statusFixture.stateSync.totalSyncedTime));
    expect(stateSyncStatus.getRemainingTime())
      .to.equal(BigInt(statusFixture.stateSync.remainingTime));
    expect(stateSyncStatus.getTotalSnapshots())
      .to.equal(statusFixture.stateSync.totalSnapshots);
    expect(stateSyncStatus.getChunkProcessAverageTime())
      .to.equal(BigInt(statusFixture.stateSync.chunkProcessAverageTime));
    expect(stateSyncStatus.getSnapshotHeight())
      .to.equal(BigInt(statusFixture.stateSync.snapshotHeight));
    expect(stateSyncStatus.getSnapshotChunkCount())
      .to.equal(BigInt(statusFixture.stateSync.snapshotChunksCount));
    expect(stateSyncStatus.getBackfilledBlocks())
      .to.equal(BigInt(statusFixture.stateSync.backfilledBlocks));
    expect(stateSyncStatus.getBackfilledBlockTotal())
      .to.equal(BigInt(statusFixture.stateSync.backfillBlocksTotal));

    expect(timeStatus).to.be.an.instanceOf(TimeStatus);
    expect(timeStatus.getLocalTime()).to.equal(BigInt(statusFixture.time.local));
    expect(timeStatus.getBlockTime()).to.equal(BigInt(statusFixture.time.block));
    expect(timeStatus.getGenesisTime()).to.equal(BigInt(statusFixture.time.genesis));
    expect(timeStatus.getEpochNumber()).to.equal(statusFixture.time.epoch);
  });

  it('should return when some fields are optional', async () => {
    response.getV0().getChain().clearCoreChainLockedHeight();
    response.getV0().getNode().clearProTxHash();
    response.getV0().getTime().clearEpoch();
    response.getV0().getVersion().getSoftware().clearDrive();
    response.getV0().getVersion().getSoftware().clearTenderdash();

    const result = await getStatus(options);

    const { GetStatusRequestV0 } = GetStatusRequest;
    const request = new GetStatusRequest();
    request.setV0(
      new GetStatusRequestV0(),
    );

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getStatus',
      request,
      options,
    );
    const versionStatus = result.getVersionStatus();
    const nodeStatus = result.getNodeStatus();
    const chainStatus = result.getChainStatus();
    const networkStatus = result.getNetworkStatus();
    const stateSyncStatus = result.getStateSyncStatus();
    const timeStatus = result.getTimeStatus();

    expect(versionStatus).to.be.an.instanceOf(VersionStatus);
    expect(versionStatus.getDriveVersion())
      .to.be.null();
    expect(versionStatus.getTenderdashVersion())
      .to.be.null();
    expect(versionStatus.getTenderdashP2pProtocol())
      .to.equal(statusFixture.version.protocol.tenderdash.p2p);
    expect(versionStatus.getTenderdashBlockProtocol())
      .to.equal(statusFixture.version.protocol.tenderdash.block);
    expect(versionStatus.getDriveCurrentProtocol())
      .to.equal(statusFixture.version.protocol.drive.current);
    expect(versionStatus.getDriveLatestProtocol())
      .to.equal(statusFixture.version.protocol.drive.latest);

    expect(nodeStatus).to.be.an.instanceOf(NodeStatus);
    expect(nodeStatus.getNodeId()).to.equal(Buffer.from(statusFixture.node.id).toString('hex'));
    expect(nodeStatus.getProTxHash()).to.be.null();

    expect(chainStatus).to.be.an.instanceOf(ChainStatus);
    expect(chainStatus.isCatchingUp())
      .to.equal(statusFixture.chain.catchingUp);
    expect(chainStatus.getLatestBlockHash())
      .to.equal(Buffer.from(statusFixture.chain.latestBlockHash).toString('hex'));
    expect(chainStatus.getLatestAppHash())
      .to.equal(Buffer.from(statusFixture.chain.latestAppHash).toString('hex'));
    expect(chainStatus.getLatestBlockHeight())
      .to.equal(BigInt(statusFixture.chain.latestBlockHeight));
    expect(chainStatus.getEarliestBlockHash())
      .to.equal(Buffer.from(statusFixture.chain.earliestBlockHash).toString('hex'));
    expect(chainStatus.getEarliestAppHash())
      .to.equal(Buffer.from(statusFixture.chain.earliestAppHash).toString('hex'));
    expect(chainStatus.getEarliestBlockHeight())
      .to.equal(BigInt(statusFixture.chain.earliestBlockHeight));
    expect(chainStatus.getMaxPeerBlockHeight())
      .to.equal(BigInt(statusFixture.chain.maxPeerBlockHeight));
    expect(chainStatus.getCoreChainLockedHeight())
      .be.null();

    expect(networkStatus).to.be.an.instanceOf(NetworkStatus);
    expect(networkStatus.getChainId()).to.equal(statusFixture.network.chainId);
    expect(networkStatus.getPeersCount()).to.equal(statusFixture.network.peersCount);
    expect(networkStatus.isListening()).to.equal(statusFixture.network.listening);

    expect(stateSyncStatus).to.be.an.instanceOf(StateSyncStatus);
    expect(stateSyncStatus.getTotalSyncedTime())
      .to.equal(BigInt(statusFixture.stateSync.totalSyncedTime));
    expect(stateSyncStatus.getRemainingTime())
      .to.equal(BigInt(statusFixture.stateSync.remainingTime));
    expect(stateSyncStatus.getTotalSnapshots())
      .to.equal(statusFixture.stateSync.totalSnapshots);
    expect(stateSyncStatus.getChunkProcessAverageTime())
      .to.equal(BigInt(statusFixture.stateSync.chunkProcessAverageTime));
    expect(stateSyncStatus.getSnapshotHeight())
      .to.equal(BigInt(statusFixture.stateSync.snapshotHeight));
    expect(stateSyncStatus.getSnapshotChunkCount())
      .to.equal(BigInt(statusFixture.stateSync.snapshotChunksCount));
    expect(stateSyncStatus.getBackfilledBlocks())
      .to.equal(BigInt(statusFixture.stateSync.backfilledBlocks));
    expect(stateSyncStatus.getBackfilledBlockTotal())
      .to.equal(BigInt(statusFixture.stateSync.backfillBlocksTotal));

    expect(timeStatus).to.be.an.instanceOf(TimeStatus);
    expect(timeStatus.getLocalTime()).to.equal(BigInt(statusFixture.time.local));
    expect(timeStatus.getBlockTime()).to.equal(BigInt(statusFixture.time.block));
    expect(timeStatus.getGenesisTime()).to.equal(BigInt(statusFixture.time.genesis));
    expect(timeStatus.getEpochNumber()).to.be.null();
  });
});
