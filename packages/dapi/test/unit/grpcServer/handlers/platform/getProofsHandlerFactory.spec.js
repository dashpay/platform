const {
  v0: {
    GetProofsResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getProofsHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getProofsHandlerFactory');

const {
  GetProofsResponseV0,
} = GetProofsResponse;

describe('getProofsHandlerFactory', () => {
  let call;
  let getProofsHandler;
  let driveClientMock;
  let request;
  let proofFixture;
  let proofMock;
  let response;

  beforeEach(async function beforeEach() {
    request = {
      getProve: this.sinon.stub().returns(true),
    };

    call = new GrpcCallMock(this.sinon, request);

    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    response = new GetProofsResponse()
      .setV0(new GetProofsResponseV0().setProof(proofMock));

    driveClientMock = {
      fetchProofs: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getProofsHandler = getProofsHandlerFactory(driveClientMock);
  });

  it('should return proof', async () => {
    const result = await getProofsHandler(call);

    expect(result).to.be.an.instanceOf(GetProofsResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveClientMock.fetchProofs).to.be.calledOnceWith(call.request);
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    driveClientMock.fetchProofs.throws(abciResponseError);

    try {
      await getProofsHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
