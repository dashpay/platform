const {
  v0: {
    PlatformPromiseClient,
    GetIdentitiesByPublicKeyHashesRequest,
    GetIdentitiesByPublicKeyHashesResponse,
    ResponseMetadata,
    Proof: ProofResponse,
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');

const getIdentitiesByPublicKeyHashesFactory = require(
  '../../../../../lib/methods/platform/getIdentitiesByPublicKeyHashes/getIdentitiesByPublicKeyHashesFactory',
);
const Proof = require('../../../../../lib/methods/platform/response/Proof');

describe('getIdentitiesByPublicKeyHashesFactory', () => {
  let grpcTransportMock;
  let getIdentitiesByPublicKeyHashes;
  let options;
  let response;
  let identityFixture;
  let publicKeyHash;
  let metadataFixture;
  let proofFixture;
  let proofResponse;
  let storeTreeProofsProto;

  beforeEach(function beforeEach() {
    identityFixture = getIdentityFixture();
    metadataFixture = getMetadataFixture();
    proofFixture = getProofFixture();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    response = new GetIdentitiesByPublicKeyHashesResponse();
    response.setIdentitiesList(
      [identityFixture.toBuffer()],
    );
    response.setMetadata(metadata);

    proofResponse = new ProofResponse();
    storeTreeProofsProto = new StoreTreeProofs();
    storeTreeProofsProto.setIdentitiesProof(proofFixture.storeTreeProofs.identitiesProof);
    storeTreeProofsProto.setPublicKeyHashesToIdentityIdsProof(
      proofFixture.storeTreeProofs.publicKeyHashesToIdentityIdsProof,
    );
    storeTreeProofsProto.setDataContractsProof(proofFixture.storeTreeProofs.dataContractsProof);
    storeTreeProofsProto.setDocumentsProof(proofFixture.storeTreeProofs.documentsProof);
    proofResponse.setSignatureLlmqHash(proofFixture.signatureLLMQHash);
    proofResponse.setSignature(proofFixture.signature);
    proofResponse.setRootTreeProof(proofFixture.rootTreeProof);
    proofResponse.setStoreTreeProofs(storeTreeProofsProto);

    publicKeyHash = identityFixture.getPublicKeyById(1).hash();

    grpcTransportMock = {
      request: this.sinon.stub().resolves(response),
    };

    options = {
      timeout: 1000,
    };

    getIdentitiesByPublicKeyHashes = getIdentitiesByPublicKeyHashesFactory(grpcTransportMock);
  });

  it('should return public key hashes to identity map', async () => {
    const result = await getIdentitiesByPublicKeyHashes([publicKeyHash], options);

    const request = new GetIdentitiesByPublicKeyHashesRequest();
    request.setPublicKeyHashesList([publicKeyHash]);
    request.setProve(false);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentitiesByPublicKeyHashes',
      request,
      options,
    );
    expect(result.getIdentities()).to.have.deep.members([identityFixture.toBuffer()]);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getProof()).to.equal(undefined);
  });

  it('should return proof', async () => {
    options.prove = true;
    response.setProof(proofResponse);
    response.setIdentitiesList([]);

    const result = await getIdentitiesByPublicKeyHashes([publicKeyHash], options);

    const request = new GetIdentitiesByPublicKeyHashesRequest();
    request.setPublicKeyHashesList([publicKeyHash]);
    request.setProve(true);

    expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
      PlatformPromiseClient,
      'getIdentitiesByPublicKeyHashes',
      request,
      options,
    );
    expect(result.getIdentities()).to.have.deep.members([]);

    expect(result.getMetadata()).to.deep.equal(metadataFixture);

    expect(result.getProof()).to.be.an.instanceOf(Proof);
    expect(result.getProof().getRootTreeProof()).to.deep.equal(proofFixture.rootTreeProof);
    expect(result.getProof().getStoreTreeProofs()).to.deep.equal(proofFixture.storeTreeProofs);
    expect(result.getProof().getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(result.getProof().getSignature()).to.deep.equal(proofFixture.signature);
    expect(result.getMetadata()).to.deep.equal(metadataFixture);
    expect(result.getMetadata().getHeight()).to.equal(metadataFixture.height);
    expect(result.getMetadata().getCoreChainLockedHeight()).to.equal(
      metadataFixture.coreChainLockedHeight,
    );
  });

  it('should throw unknown error', async () => {
    const error = new Error('Unknown found');

    grpcTransportMock.request.throws(error);

    const request = new GetIdentitiesByPublicKeyHashesRequest();
    request.setPublicKeyHashesList([publicKeyHash]);
    request.setProve(false);

    try {
      await getIdentitiesByPublicKeyHashes([publicKeyHash], options);

      expect.fail('should throw unknown error');
    } catch (e) {
      expect(e).to.deep.equal(error);
      expect(grpcTransportMock.request).to.be.calledOnceWithExactly(
        PlatformPromiseClient,
        'getIdentitiesByPublicKeyHashes',
        request,
        options,
      );
    }
  });
});
