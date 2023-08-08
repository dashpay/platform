const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityByPublicKeyHashesResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

/* eslint-disable import/no-extraneous-dependencies */
const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

const getIdentityByPublicKeyHashesHandlerFactory = require('../../../../../lib/grpcServer/handlers/platform/getIdentityByPublicKeyHashesHandlerFactory');

describe('getIdentityByPublicKeyHashesHandlerFactory', () => {
  let call;
  let getIdentityByPublicKeyHashesHandler;
  let driveClientMock;
  let request;
  let proofFixture;
  let proofMock;
  let response;
  let proofResponse;
  let identityFixture;

  beforeEach(async function beforeEach() {
    request = {
      getPublicKeyHash: this.sinon.stub().returns(
        Buffer.alloc(1, 1),
      ),
      getProve: this.sinon.stub().returns(true),
    };

    identityFixture = await getIdentityFixture();

    call = new GrpcCallMock(this.sinon, request);

    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    response = new GetIdentityByPublicKeyHashesResponse();
    response.setIdentity(identityFixture.toBuffer());

    proofResponse = new GetIdentityByPublicKeyHashesResponse();
    proofResponse.setProof(proofMock);

    driveClientMock = {
      fetchIdentityByPublicKeyHashes: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getIdentityByPublicKeyHashesHandler = getIdentityByPublicKeyHashesHandlerFactory(
      driveClientMock,
    );
  });

  it('should return identity', async () => {
    const result = await getIdentityByPublicKeyHashesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityByPublicKeyHashesResponse);

    const identity = result.getIdentity();

    expect(identity).to.deep.equal(identityFixture.toBuffer());

    const proof = result.getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async function it() {
    driveClientMock = {
      fetchIdentityByPublicKeyHashes: this.sinon.stub().resolves(
        proofResponse.serializeBinary(),
      ),
    };

    getIdentityByPublicKeyHashesHandler = getIdentityByPublicKeyHashesHandlerFactory(
      driveClientMock,
    );

    const result = await getIdentityByPublicKeyHashesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentityByPublicKeyHashesResponse);

    const identity = result.getIdentity();
    expect(identity).to.be.equal('');

    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);

    expect(driveClientMock.fetchIdentityByPublicKeyHashes).to.be.calledOnceWith(
      call.request,
    );
  });

  it('should throw InvalidArgumentGrpcError error if ids are not specified', async () => {
    request.getPublicKeyHash.returns([]);

    try {
      await getIdentityByPublicKeyHashesHandler(call);

      expect.fail('should thrown InvalidArgumentGrpcError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('No public key hash is provided');
      expect(driveClientMock.fetchIdentityByPublicKeyHashes).to.be.not.called();
    }
  });

  it('should throw error if driveStateRepository throws an error', async () => {
    const message = 'Some error';
    const abciResponseError = new Error(message);

    driveClientMock.fetchIdentityByPublicKeyHashes.throws(abciResponseError);

    try {
      await getIdentityByPublicKeyHashesHandler(call);

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(abciResponseError);
    }
  });
});
