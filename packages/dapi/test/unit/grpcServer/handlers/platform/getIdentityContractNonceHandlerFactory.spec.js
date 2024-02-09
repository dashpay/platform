const {
  v0: {
    GetIdentityContractNonceResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityContractNonceHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityContractNonceHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getIdentityContractNonceHandlerFactory', () => {
  let call;
  let driveStateRepositoryMock;
  let getIdentityContractNonceHandler;
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
    const { GetIdentityContractNonceResponseV0 } = GetIdentityContractNonceResponse;
    response = new GetIdentityContractNonceResponse();
    response.setV0(
      new GetIdentityContractNonceResponseV0()
        .setIdentityContractNonce(1),
    );

    proofResponse = new GetIdentityContractNonceResponse();
    proofResponse.setV0(
      new GetIdentityContractNonceResponseV0()
        .setProof(proofMock),
    );

    driveStateRepositoryMock = {
      fetchIdentityContractNonceRequest: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getIdentityContractNonceHandler = getIdentityContractNonceHandlerFactory(
      driveStateRepositoryMock,
    );
  });

  it('should return valid result', async () => {
    const result = await getIdentityContractNonceHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityContractNonceResponse);
    expect(result.getV0()
      .getIdentityContractNonce()).to.deep.equal(nonce);
    expect(driveStateRepositoryMock.fetchIdentityContractNonceRequest)
      .to.be.calledOnceWith(call.request);

    const proof = result.getV0().getProof();
    expect(proof).to.be.undefined();
  });

  it('should return proof', async () => {
    request.getProve.returns(true);

    driveStateRepositoryMock.fetchIdentityContractNonceRequest
      .resolves(proofResponse.serializeBinary());

    const result = await getIdentityContractNonceHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityContractNonceResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveStateRepositoryMock.fetchIdentityContractNonceRequest)
      .to.be.calledOnceWith(call.request);
  });

  it('should throw an error when fetchIdentityContractNonceRequest throws unknown error', async () => {
    const error = new Error('Unknown error');

    driveStateRepositoryMock.fetchIdentityContractNonceRequest.throws(error);

    try {
      await getIdentityContractNonceHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveStateRepositoryMock.fetchIdentityContractNonceRequest)
        .to.be.calledOnceWith(call.request);
    }
  });
});
