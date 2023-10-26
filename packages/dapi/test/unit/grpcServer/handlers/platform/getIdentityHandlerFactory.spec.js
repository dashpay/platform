const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

/* eslint-disable import/no-extraneous-dependencies */
const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');

const getIdentityHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityHandlerFactory');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getIdentityHandlerFactory', () => {
  let call;
  let driveStateRepositoryMock;
  let id;
  let getIdentityHandler;
  let identity;
  let proofFixture;
  let proofMock;
  let request;
  let response;
  let proofResponse;

  beforeEach(async function beforeEach() {
    id = await generateRandomIdentifierAsync();
    request = {
      getId: this.sinon.stub().returns(id),
      getProve: this.sinon.stub().returns(false),
    };
    call = new GrpcCallMock(this.sinon, {
      getV0: () => request,
    });

    identity = await getIdentityFixture();

    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    response = new GetIdentityResponse();
    response.setV0(
      new GetIdentityResponse.GetIdentityResponseV0()
        .setIdentity(identity.toBuffer()),
    );

    proofResponse = new GetIdentityResponse();
    proofResponse.setV0(
      new GetIdentityResponse.GetIdentityResponseV0()
        .setProof(proofMock),
    );

    driveStateRepositoryMock = {
      fetchIdentity: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getIdentityHandler = getIdentityHandlerFactory(
      driveStateRepositoryMock,
    );
  });

  it('should return valid result', async () => {
    const result = await getIdentityHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityResponse);
    expect(result.getV0().getIdentity()).to.deep.equal(identity.toBuffer());
    expect(driveStateRepositoryMock.fetchIdentity).to.be.calledOnceWith(call.request);

    const proof = result.getV0().getProof();
    expect(proof).to.be.undefined();
  });

  it('should return proof', async () => {
    request.getProve.returns(true);

    driveStateRepositoryMock.fetchIdentity.resolves(proofResponse.serializeBinary());

    const result = await getIdentityHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveStateRepositoryMock.fetchIdentity).to.be.calledOnceWith(call.request);
  });

  it('should throw an InvalidArgumentGrpcError if id is not specified', async () => {
    request.getId.returns(null);

    try {
      await getIdentityHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('id is not specified');
      expect(driveStateRepositoryMock.fetchIdentity).to.not.be.called();
    }
  });

  it('should throw an error when fetchIdentity throws unknown error', async () => {
    const error = new Error('Unknown error');

    driveStateRepositoryMock.fetchIdentity.throws(error);

    try {
      await getIdentityHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveStateRepositoryMock.fetchIdentity).to.be.calledOnceWith(call.request);
    }
  });
});
