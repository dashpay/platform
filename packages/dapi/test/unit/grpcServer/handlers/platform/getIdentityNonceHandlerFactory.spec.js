const {
  v0: {
    GetIdentityNonceResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityNonceHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityNonceHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getIdentityNonceHandlerFactory', () => {
  let call;
  let driveStateRepositoryMock;
  let getIdentityNonceHandler;
  let nonce;
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

    nonce = 1;
    const { GetIdentityNonceResponseV0 } = GetIdentityNonceResponse;
    response = new GetIdentityNonceResponse();
    response.setV0(
      new GetIdentityNonceResponseV0()
        .setIdentityNonce(1),
    );

    proofResponse = new GetIdentityNonceResponse();
    proofResponse.setV0(
      new GetIdentityNonceResponseV0()
        .setProof(proofMock),
    );

    driveStateRepositoryMock = {
      fetchIdentityNonce: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getIdentityNonceHandler = getIdentityNonceHandlerFactory(
      driveStateRepositoryMock,
    );
  });

  it('should return valid result', async () => {
    const result = await getIdentityNonceHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityNonceResponse);
    expect(result.getV0()
      .getIdentityNonce()).to.deep.equal(nonce);
    expect(driveStateRepositoryMock.fetchIdentityNonce)
      .to.be.calledOnceWith(call.request);

    const proof = result.getV0().getProof();
    expect(proof).to.be.undefined();
  });

  it('should return proof', async () => {
    request.getProve.returns(true);

    driveStateRepositoryMock.fetchIdentityNonce
      .resolves(proofResponse.serializeBinary());

    const result = await getIdentityNonceHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityNonceResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveStateRepositoryMock.fetchIdentityNonce)
      .to.be.calledOnceWith(call.request);
  });

  it('should throw an error when fetchIdentityNonce throws unknown error', async () => {
    const error = new Error('Unknown error');

    driveStateRepositoryMock.fetchIdentityNonce.throws(error);

    try {
      await getIdentityNonceHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveStateRepositoryMock.fetchIdentityNonce)
        .to.be.calledOnceWith(call.request);
    }
  });
});
