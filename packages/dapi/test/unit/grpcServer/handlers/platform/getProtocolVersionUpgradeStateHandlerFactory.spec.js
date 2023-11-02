const {
  v0: {
    GetProtocolVersionUpgradeStateResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const getProtocolVersionUpgradeStateHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getProtocolVersionUpgradeStateHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getProtocolVersionUpgradeStateHandlerFactory', () => {
  let call;
  let driveStateRepositoryMock;
  let getProtocolVersionUpgradeStateHandler;
  let versionNumber;
  let voteCount;
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

    versionNumber = 1;
    voteCount = 1;
    const { GetProtocolVersionUpgradeStateResponseV0 } = GetProtocolVersionUpgradeStateResponse;
    const { Versions, VersionEntry } = GetProtocolVersionUpgradeStateResponseV0;
    response = new GetProtocolVersionUpgradeStateResponse();
    response.setV0(
      new GetProtocolVersionUpgradeStateResponseV0()
        .setVersions(new Versions()
          .setVersionsList([new VersionEntry()
            .setVersionNumber(versionNumber)
            .setVoteCount(voteCount)])),
    );

    proofResponse = new GetProtocolVersionUpgradeStateResponse();
    proofResponse.setV0(
      new GetProtocolVersionUpgradeStateResponseV0()
        .setProof(proofMock),
    );

    driveStateRepositoryMock = {
      fetchVersionUpgradeState: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getProtocolVersionUpgradeStateHandler = getProtocolVersionUpgradeStateHandlerFactory(
      driveStateRepositoryMock,
    );
  });

  it('should return valid result', async () => {
    const result = await getProtocolVersionUpgradeStateHandler(call);

    expect(result).to.be.an.instanceOf(GetProtocolVersionUpgradeStateResponse);
    expect(result.getV0()
      .getVersions().getVersionsList()[0].getVersionNumber()).to.deep.equal(versionNumber);
    expect(driveStateRepositoryMock.fetchVersionUpgradeState)
      .to.be.calledOnceWith(call.request);

    const proof = result.getV0().getProof();
    expect(proof).to.be.undefined();
  });

  it('should return proof', async () => {
    request.getProve.returns(true);

    driveStateRepositoryMock.fetchVersionUpgradeState
      .resolves(proofResponse.serializeBinary());

    const result = await getProtocolVersionUpgradeStateHandler(call);

    expect(result).to.be.an.instanceOf(GetProtocolVersionUpgradeStateResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveStateRepositoryMock.fetchVersionUpgradeState)
      .to.be.calledOnceWith(call.request);
  });

  it('should throw an error when fetchVersionUpgradeState throws unknown error', async () => {
    const error = new Error('Unknown error');

    driveStateRepositoryMock.fetchVersionUpgradeState.throws(error);

    try {
      await getProtocolVersionUpgradeStateHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveStateRepositoryMock.fetchVersionUpgradeState)
        .to.be.calledOnceWith(call.request);
    }
  });
});
