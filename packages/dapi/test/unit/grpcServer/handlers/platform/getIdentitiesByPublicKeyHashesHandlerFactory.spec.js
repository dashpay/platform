const { BytesValue } = require('google-protobuf/google/protobuf/wrappers_pb');

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
  let proofResponse;
  let request;

  beforeEach(async function beforeEach() {
    publicKeyHash = Buffer.from('556c2910d46fda2b327ef9d9bda850cc84d30db0', 'hex');

    request = {
      getPublicKeyHashesList: this.sinon.stub().returns(
        [publicKeyHash],
      ),
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

    const {
      IdentitiesByPublicKeyHashes,
      PublicKeyHashIdentityEntry,
      GetIdentitiesByPublicKeyHashesResponseV0,
    } = GetIdentitiesByPublicKeyHashesResponse;

    response = new GetIdentitiesByPublicKeyHashesResponse();
    response.setV0(
      new GetIdentitiesByPublicKeyHashesResponseV0().setIdentities(
        new IdentitiesByPublicKeyHashes()
          .setIdentityEntriesList([
            new PublicKeyHashIdentityEntry()
              .setPublicKeyHash(publicKeyHash)
              .setValue(new BytesValue().setValue(identity.toBuffer())),
          ]),
      ),
    );

    proofResponse = new GetIdentitiesByPublicKeyHashesResponse();
    proofResponse.setV0(
      new GetIdentitiesByPublicKeyHashesResponseV0().setProof(proofMock),
    );

    driveClientMock = {
      fetchIdentitiesByPublicKeyHashes: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getIdentitiesByPublicKeyHashesHandler = getIdentitiesByPublicKeyHashesHandlerFactory(
      driveClientMock,
    );
  });

  it('should return identities', async () => {
    const result = await getIdentitiesByPublicKeyHashesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentitiesByPublicKeyHashesResponse);

    expect(result.getV0()
      .getIdentities()
      .getIdentityEntriesList()[0]
      .getValue()
      .getValue()).to.deep.equal(
      identity.toBuffer(),
    );

    expect(driveClientMock.fetchIdentitiesByPublicKeyHashes)
      .to.be.calledOnceWith(call.request);

    const proof = result.getV0().getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async () => {
    request.getProve.returns(true);
    driveClientMock.fetchIdentitiesByPublicKeyHashes.resolves(proofResponse.serializeBinary());

    const result = await getIdentitiesByPublicKeyHashesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentitiesByPublicKeyHashesResponse);

    const proof = result.getV0().getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const merkleProof = proof.getGrovedbProof();

    expect(merkleProof).to.deep.equal(proofFixture.merkleProof);
  });

  it('should throw an InvalidArgumentGrpcError if no hashes were submitted', async () => {
    request.getPublicKeyHashesList.returns([]);

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
