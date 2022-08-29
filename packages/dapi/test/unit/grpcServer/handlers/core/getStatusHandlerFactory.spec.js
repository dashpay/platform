const {
  v0: {
    GetStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

const getStatusHandlerFactory = require('../../../../../lib/grpcServer/handlers/core/getStatusHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getStatusHandlerFactory', () => {
  let call;
  let getStatusHandler;
  let coreRPCClientMock;
  let now;

  let blockchainInfo;
  let networkInfo;
  let mnSyncInfo;
  let masternodeStatus;

  beforeEach(function beforeEach() {
    mnSyncInfo = {
      AssetID: 999,
      AssetName: 'MASTERNODE_SYNC_FINISHED',
      AssetStartTime: 1615466139,
      Attempt: 0,
      IsBlockchainSynced: true,
      IsSynced: true,
    };

    blockchainInfo = {
      chain: 'test',
      blocks: 460991,
      headers: 460991,
      bestblockhash: '0000007464fd8cae97830d794bf03efbeaa4b8c3258a3def67a89cdbd060f827',
      difficulty: 0.002261509525429119,
      mediantime: 1615546573,
      verificationprogress: 0.9999993798366165,
      initialblockdownload: false,
      chainwork: '000000000000000000000000000000000000000000000000022f149b98e063dc',
      warnings: 'Warning: unknown new rules activated (versionbit 3)',
    };

    networkInfo = {
      version: 170000,
      subversion: '/Dash Core:0.17.0/',
      protocolversion: 70218,
      localservices: '0000000000000405',
      localrelay: true,
      timeoffset: 0,
      networkactive: true,
      connections: 8,
      socketevents: 'select',
      relayfee: 0.00001,
      incrementalfee: 0.00001,
      warnings: 'Warning: unknown new rules activated (versionbit 3)',
    };

    masternodeStatus = {
      outpoint: 'd1be3a1aa0b9516d06ed180607c168724c21d8ccf6c5a3f5983769830724c357-0',
      service: '45.32.237.76:19999',
      proTxHash: '04d06d16b3eca2f104ef9749d0c1c17d183eb1b4fe3a16808fd70464f03bcd63',
      collateralHash: 'd1be3a1aa0b9516d06ed180607c168724c21d8ccf6c5a3f5983769830724c357',
      collateralIndex: 0,
      dmnState: {
        service: '45.32.237.76:19999',
        registeredHeight: 7402,
        lastPaidHeight: 59721,
        PoSePenalty: 0,
        PoSeRevivedHeight: 61915,
        PoSeBanHeight: -1,
        revocationReason: 0,
        ownerAddress: 'yT8DDY5NkX4ZtBkUVz7y1RgzbakCnMPogh',
        votingAddress: 'yMLrhooXyJtpV3R2ncsxvkrh6wRennNPoG',
        payoutAddress: 'yTsGq4wV8WF5GKLaYV2C43zrkr2sfTtysT',
        pubKeyOperator: '02a2e2673109a5e204f8a82baf628bb5f09a8dfc671859e84d2661cae03e6c6e198a037e968253e94cd099d07b98e94e',
      },
      state: 'READY',
      status: 'Ready',
    };

    call = new GrpcCallMock(this.sinon);

    coreRPCClientMock = {
      getBlockchainInfo: this.sinon.stub().resolves(blockchainInfo),
      getNetworkInfo: this.sinon.stub().resolves(networkInfo),
      getMnSync: this.sinon.stub().resolves(mnSyncInfo),
      getMasternode: this.sinon.stub().resolves(masternodeStatus),
    };

    now = new Date();
    this.sinon.useFakeTimers(now.getTime());

    getStatusHandler = getStatusHandlerFactory(coreRPCClientMock);
  });

  it('should return valid result', async () => {
    const result = await getStatusHandler(call);

    expect(result).to.be.an.instanceOf(GetStatusResponse);

    // Validate protobuf object values
    result.serializeBinary();

    const version = result.getVersion();
    expect(version.getProtocol()).to.equal(networkInfo.protocolversion);
    expect(version.getSoftware()).to.equal(networkInfo.version);
    expect(version.getAgent()).to.equal(networkInfo.subversion);

    const time = result.getTime();
    expect(time.getNow()).to.be.an('number');
    expect(time.getNow()).to.equal(Math.floor(now.getTime() / 1000));
    expect(time.getOffset()).to.be.equal(networkInfo.timeoffset);
    expect(time.getMedian()).to.be.equal(blockchainInfo.mediantime);

    const chain = result.getChain();
    expect(chain.getName()).to.be.equal(blockchainInfo.chain);
    expect(chain.getBlocksCount()).to.be.equal(blockchainInfo.blocks);
    expect(chain.getHeadersCount()).to.be.equal(blockchainInfo.headers);
    expect(chain.getBestBlockHash()).to.be.an.instanceOf(Buffer);
    expect(chain.getBestBlockHash().toString('hex')).to.be.equal(blockchainInfo.bestblockhash);
    expect(chain.getDifficulty()).to.be.equal(blockchainInfo.difficulty);
    expect(chain.getChainWork()).to.be.an.instanceOf(Buffer);
    expect(chain.getChainWork().toString('hex')).to.be.equal(blockchainInfo.chainwork);
    expect(chain.getIsSynced()).to.be.equal(mnSyncInfo.IsBlockchainSynced);
    expect(chain.getSyncProgress()).to.be.equal(blockchainInfo.verificationprogress);

    const masternode = result.getMasternode();
    expect(masternode.getStatus()).to.be.equal(GetStatusResponse.Masternode.Status.READY);
    expect(masternode.getProTxHash()).to.be.an.instanceOf(Buffer);
    expect(masternode.getProTxHash().toString('hex')).to.be.equal(masternodeStatus.proTxHash);
    expect(masternode.getPosePenalty()).to.be.equal(masternodeStatus.dmnState.PoSePenalty);
    expect(masternode.getIsSynced()).to.be.equal(mnSyncInfo.IsSynced);
    expect(masternode.getSyncProgress()).to.be.equal(1);

    const network = result.getNetwork();
    expect(network.getPeersCount()).to.be.equal(networkInfo.connections);

    const fee = network.getFee();
    expect(fee.getRelay()).to.be.equal(networkInfo.relayfee);
    expect(fee.getIncremental()).to.be.equal(networkInfo.incrementalfee);

    expect(result.getStatus()).to.be.equal(GetStatusResponse.Status.READY);
    expect(coreRPCClientMock.getBlockchainInfo).to.be.calledOnce();
    expect(coreRPCClientMock.getNetworkInfo).to.be.calledOnce();
    expect(coreRPCClientMock.getMnSync).to.be.calledOnceWith('status');
    expect(coreRPCClientMock.getMasternode).to.be.calledOnceWith('status');
  });
});
