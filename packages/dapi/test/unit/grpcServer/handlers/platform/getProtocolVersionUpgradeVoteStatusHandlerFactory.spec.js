const {
  v0: {
    GetProtocolVersionUpgradeVoteStatusResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const getProtocolVersionUpgradeVoteStatusHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getProtocolVersionUpgradeVoteStatusHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getProtocolVersionUpgradeVoteStatusHandlerFactory', () => {
  let call;
  let driveStateRepositoryMock;
  let getProtocolVersionUpgradeVoteStatusHandler;
  let proTxHash;
  let version;
  let proofFixture;
  let proofMock;
  let request;
  let response;
  let proofResponse;

  beforeEach(async function beforeEach() {
    request = {
      getProve: this.sinon.stub().returns(false),
    };
    call = new GrpcCallMock(this.sinon, {
      getV0: () => request,
    });

    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    version = 1;
    proTxHash = Buffer.alloc(32).fill(1);
    const {
      GetProtocolVersionUpgradeVoteStatusResponseV0,
    } = GetProtocolVersionUpgradeVoteStatusResponse;
    const { VersionSignals, VersionSignal } = GetProtocolVersionUpgradeVoteStatusResponseV0;
    response = new GetProtocolVersionUpgradeVoteStatusResponse();
    response.setV0(
      new GetProtocolVersionUpgradeVoteStatusResponseV0()
        .setVersions(new VersionSignals()
          .setVersionSignalsList([new VersionSignal()
            .setProTxHash(proTxHash)
            .setVersion(version)])),
    );

    proofResponse = new GetProtocolVersionUpgradeVoteStatusResponse();
    proofResponse.setV0(
      new GetProtocolVersionUpgradeVoteStatusResponseV0()
        .setProof(proofMock),
    );

    driveStateRepositoryMock = {
      fetchVersionUpgradeVoteStatus: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getProtocolVersionUpgradeVoteStatusHandler = getProtocolVersionUpgradeVoteStatusHandlerFactory(
      driveStateRepositoryMock,
    );
  });

  it('should return valid result', async () => {
    const result = await getProtocolVersionUpgradeVoteStatusHandler(call);

    expect(result).to.be.an.instanceOf(GetProtocolVersionUpgradeVoteStatusResponse);
    expect(result.getV0()
      .getVersions().getVersionSignalsList()[0].getProTxHash()).to.deep.equal(proTxHash);
    expect(driveStateRepositoryMock.fetchVersionUpgradeVoteStatus)
      .to.be.calledOnceWith(call.request);

    const proof = result.getV0().getProof();
    expect(proof).to.be.undefined();
  });

  it('should return proof', async () => {
    request.getProve.returns(true);

    driveStateRepositoryMock.fetchVersionUpgradeVoteStatus
      .resolves(proofResponse.serializeBinary());

    const result = await getProtocolVersionUpgradeVoteStatusHandler(call);

    expect(result).to.be.an.instanceOf(GetProtocolVersionUpgradeVoteStatusResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveStateRepositoryMock.fetchVersionUpgradeVoteStatus)
      .to.be.calledOnceWith(call.request);
  });

  it('should throw an error when fetchVersionUpgradeVoteStatus throws unknown error', async () => {
    const error = new Error('Unknown error');

    driveStateRepositoryMock.fetchVersionUpgradeVoteStatus.throws(error);

    try {
      await getProtocolVersionUpgradeVoteStatusHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveStateRepositoryMock.fetchVersionUpgradeVoteStatus)
        .to.be.calledOnceWith(call.request);
    }
  });
});
