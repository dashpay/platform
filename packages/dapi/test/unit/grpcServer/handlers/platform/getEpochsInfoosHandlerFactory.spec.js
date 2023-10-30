const {
  v0: {
    GetEpochsInfoResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const getEpochInfosHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getEpochInfosHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getEpochInfosHandlerFactory', () => {
  let call;
  let driveStateRepositoryMock;
  let getEpochInfosHandler;
  let epochNumber;
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

    epochNumber = 1;
    const { GetEpochsInfoResponseV0 } = GetEpochsInfoResponse;
    const { EpochInfos, EpochInfo } = GetEpochsInfoResponseV0;
    response = new GetEpochsInfoResponse();
    response.setV0(
      new GetEpochsInfoResponseV0()
        .setEpochs(new EpochInfos()
          .setEpochInfosList([new EpochInfo()
            .setNumber(epochNumber)
            .setFirstBlockHeight(1)
            .setFirstCoreBlockHeight(1)
            .setStartTime(Date.now())
            .setFeeMultiplier(1.1)])),
    );

    proofResponse = new GetEpochsInfoResponse();
    proofResponse.setV0(
      new GetEpochsInfoResponseV0()
        .setProof(proofMock),
    );

    driveStateRepositoryMock = {
      fetchEpochInfos: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getEpochInfosHandler = getEpochInfosHandlerFactory(
      driveStateRepositoryMock,
    );
  });

  it('should return valid result', async () => {
    const result = await getEpochInfosHandler(call);

    expect(result).to.be.an.instanceOf(GetEpochsInfoResponse);
    expect(result.getV0()
      .getEpochs().getEpochInfosList()[0].getNumber()).to.equal(epochNumber);
    expect(driveStateRepositoryMock.fetchEpochInfos).to.be.calledOnceWith(call.request);

    const proof = result.getV0().getProof();
    expect(proof).to.be.undefined();
  });

  it('should return proof', async () => {
    request.getProve.returns(true);

    driveStateRepositoryMock.fetchEpochInfos.resolves(proofResponse.serializeBinary());

    const result = await getEpochInfosHandler(call);

    expect(result).to.be.an.instanceOf(GetEpochsInfoResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveStateRepositoryMock.fetchEpochInfos).to.be.calledOnceWith(call.request);
  });

  it('should throw an error when fetchEpochInfos throws unknown error', async () => {
    const error = new Error('Unknown error');

    driveStateRepositoryMock.fetchEpochInfos.throws(error);

    try {
      await getEpochInfosHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveStateRepositoryMock.fetchEpochInfos).to.be.calledOnceWith(call.request);
    }
  });
});
