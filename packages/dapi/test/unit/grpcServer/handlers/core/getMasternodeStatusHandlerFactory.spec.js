const {
  v0: {
    GetMasternodeStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

const getMasternodeStatusHandlerFactory = require('../../../../../lib/grpcServer/handlers/core/getMasternodeStatusHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getMasternodeStatusHandlerFactory', () => {
  let call;
  let getMasternodeStatusHandler;
  let coreRPCClientMock;
  let now;

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
      getMnSync: this.sinon.stub().resolves(mnSyncInfo),
      getMasternode: this.sinon.stub().resolves(masternodeStatus),
    };

    now = new Date();
    this.sinon.useFakeTimers(now.getTime());

    getMasternodeStatusHandler = getMasternodeStatusHandlerFactory(coreRPCClientMock);
  });

  it('should return valid result', async () => {
    const result = await getMasternodeStatusHandler(call);

    expect(result).to.be.an.instanceOf(GetMasternodeStatusResponse);

    // Validate protobuf object values
    result.serializeBinary();

    expect(result.getStatus()).to.be.equal(GetMasternodeStatusResponse.Status.READY);
    expect(result.getProTxHash()).to.be.an.instanceOf(Buffer);
    expect(result.getProTxHash().toString('hex')).to.be.equal(masternodeStatus.proTxHash);
    expect(result.getPosePenalty()).to.be.equal(masternodeStatus.dmnState.PoSePenalty);
    expect(result.getIsSynced()).to.be.equal(mnSyncInfo.IsSynced);
    expect(result.getSyncProgress()).to.be.equal(1);

    expect(coreRPCClientMock.getMnSync).to.be.calledOnceWith('status');
    expect(coreRPCClientMock.getMasternode).to.be.calledOnceWith('status');
  });
});
