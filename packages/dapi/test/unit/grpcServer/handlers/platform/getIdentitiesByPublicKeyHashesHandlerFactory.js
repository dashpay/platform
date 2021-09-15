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
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const getIdentitiesByPublicKeyHashesHandlerFactory = require(
  '../../../../../lib/grpcServer/handlers/platform/getIdentitiesByPublicKeyHashesHandlerFactory',
);

const GrpcCallMock = require('../../../../../lib/test/mock/GrpcCallMock');

describe('getIdentitiesByPublicKeyHashesHandlerFactory', () => {
  let call;
  let driveStateRepositoryMock;
  let getIdentitiesByPublicKeyHashesHandler;
  let identity;
  let publicKeyHash;
  let proofFixture;
  let proofMock;
  let response;
  let storeTreeProofs;

  beforeEach(function beforeEach() {
    publicKeyHash = '556c2910d46fda2b327ef9d9bda850cc84d30db0';

    call = new GrpcCallMock(this.sinon, {
      getPublicKeyHashesList: this.sinon.stub().returns(
        [publicKeyHash],
      ),
      getProve: this.sinon.stub().returns(false),
    });

    identity = getIdentityFixture();

    proofFixture = {
      rootTreeProof: Buffer.alloc(1, 1),
      storeTreeProof: Buffer.alloc(1, 2),
    };

    storeTreeProofs = new StoreTreeProofs();
    storeTreeProofs.setDataContractsProof(proofFixture.storeTreeProof);

    proofMock = new Proof();
    proofMock.setRootTreeProof(proofFixture.rootTreeProof);
    proofMock.setStoreTreeProofs(storeTreeProofs);

    response = new GetIdentitiesByPublicKeyHashesResponse();
    response.setProof(proofMock);
    response.setIdentitiesList([identity.toBuffer()]);

    driveStateRepositoryMock = {
      fetchIdentitiesByPublicKeyHashes: this.sinon.stub().resolves(response.serializeBinary()),
    };

    getIdentitiesByPublicKeyHashesHandler = getIdentitiesByPublicKeyHashesHandlerFactory(
      driveStateRepositoryMock,
    );
  });

  it('should return valid result', async () => {
    response.setProof(null);
    driveStateRepositoryMock.fetchIdentitiesByPublicKeyHashes.resolves(response.serializeBinary());

    const result = await getIdentitiesByPublicKeyHashesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentitiesByPublicKeyHashesResponse);

    expect(result.getIdentitiesList()).to.deep.equal(
      [identity.toBuffer()],
    );

    expect(driveStateRepositoryMock.fetchIdentitiesByPublicKeyHashes)
      .to.be.calledOnceWith([publicKeyHash], false);

    const proof = result.getProof();

    expect(proof).to.be.undefined();
  });

  it('should return proof', async () => {
    call.request.getProve.returns(true);
    const result = await getIdentitiesByPublicKeyHashesHandler(call);

    expect(result).to.be.an.instanceOf(GetIdentitiesByPublicKeyHashesResponse);

    const proof = result.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    const rootTreeProof = proof.getRootTreeProof();
    const resultStoreTreeProof = proof.getStoreTreeProofs();

    expect(rootTreeProof).to.deep.equal(proofFixture.rootTreeProof);
    expect(resultStoreTreeProof).to.deep.equal(storeTreeProofs);
  });

  it('should throw an InvalidArgumentGrpcError if no hashes were submitted', async () => {
    call.request.getPublicKeyHashesList.returns([]);

    try {
      await getIdentitiesByPublicKeyHashesHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentGrpcError);
      expect(e.getMessage()).to.equal('No public key hashes were provided');
      expect(driveStateRepositoryMock.fetchIdentitiesByPublicKeyHashes).to.not.be.called();
    }
  });

  it('should throw an error when fetchIdentity throws an error', async () => {
    const error = new Error('Unknown error');

    driveStateRepositoryMock.fetchIdentitiesByPublicKeyHashes.throws(error);

    try {
      await getIdentitiesByPublicKeyHashesHandler(call);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.equal(error);
      expect(driveStateRepositoryMock.fetchIdentitiesByPublicKeyHashes)
        .to.be.calledOnceWith([publicKeyHash]);
    }
  });
});
