const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentitiesByPublicKeyHashesResponse,
    Proof,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');

const getIdentitiesByPublicKeyHashesHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/getIdentitiesByPublicKeyHashesHandlerFactory',
);

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getIdentitiesByPublicKeyHashesHandlerFactory', () => {
  let call;
  let driveClientMock;
  let getIdentitiesByPublicKeyHashesHandler;
  let identity;
  let publicKeyHash;
  let proofFixture;
  let proofMock;
  let response;

  beforeEach(async function beforeEach() {
    publicKeyHash = Buffer.from('556c2910d46fda2b327ef9d9bda850cc84d30db0', 'hex');

    call = new GrpcCallMock(this.sinon, {
      getPublicKeyHashesList: this.sinon.stub().returns(
        [publicKeyHash],
      ),
      getProve: this.sinon.stub().returns(false),
    });

    identity = await getIdentityFixture();

    proofFixture = {
      merkleProof: Buffer.alloc(1, 1),
    };

    proofMock = new Proof();
    proofMock.setGrovedbProof(proofFixture.merkleProof);

    response = new GetIdentitiesByPublicKeyHashesResponse();
    response.setProof(proofMock);
    response.setIdentitiesList([identity.toBuffer()]);

    driveClientMock = {
      fetchIdentitiesByPublicKeyHashes: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getIdentitiesByPublicKeyHashesHandler = getIdentitiesByPublicKeyHashesHandlerFactory(
      driveClientMock,
    );
  });

  it('should return valid result', async () => {
    response.setProof(null);
    driveClientMock.fetchIdentitiesByPublicKeyHashes.resolves(response.serializeBinary());

    const result = await getIdentitiesByPublicKeyHashesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentitiesByPublicKeyHashesResponse);

    expect(result.getIdentitiesList()).to.deep.equal(
      [identity.toBuffer()],
    );

    expect(driveClientMock.fetchIdentitiesByPublicKeyHashes)
      .to.be.calledOnceWith(call.request);

    const proof = result.getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async () => {
    call.request.getProve.returns(true);
    const result = await getIdentitiesByPublicKeyHashesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentitiesByPublicKeyHashesResponse);

    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);
  });

  it('should throw an InvalidArgumentGrpcError if no hashes were submitted', async () => {
    call.request.getPublicKeyHashesList.returns([]);

    try {
      await getIdentitiesByPublicKeyHashesHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('No public key hashes were provided');
      expect(driveClientMock.fetchIdentitiesByPublicKeyHashes).to.not.be.called();
    }
  });

  it('should throw an error when fetchIdentity throws an error', async () => {
    const error = new Error('Unknown error');

    driveClientMock.fetchIdentitiesByPublicKeyHashes.throws(error);

    try {
      await getIdentitiesByPublicKeyHashesHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveClientMock.fetchIdentitiesByPublicKeyHashes)
        .to.be.calledOnceWith(call.request);
    }
  });
});
