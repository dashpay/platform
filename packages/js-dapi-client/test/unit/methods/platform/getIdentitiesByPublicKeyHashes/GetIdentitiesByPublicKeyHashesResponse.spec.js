const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const {
  v0: {
    GetIdentitiesByPublicKeyHashesResponse,
    ResponseMetadata,
    Proof: ProofResponse,
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentitiesByPublicKeyHashesResponseClass = require('../../../../../lib/methods/platform/getIdentitiesByPublicKeyHashes/GetIdentitiesByPublicKeyHashesResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetIdentitiesByPublicKeyHashesResponse', () => {
  let getIdentitiesResponse;
  let metadataFixture;
  let identityFixture;
  let proto;
  let proofFixture;

  beforeEach(() => {
    metadataFixture = getMetadataFixture();
    identityFixture = getIdentityFixture();
    proofFixture = getProofFixture();

    proto = new GetIdentitiesByPublicKeyHashesResponse();

    proto.setIdentitiesList(
      [identityFixture.toBuffer()],
    );

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    getIdentitiesResponse = new GetIdentitiesByPublicKeyHashesResponseClass(
      [identityFixture.toBuffer()],
      new Metadata(metadataFixture),
    );
  });

  it('should return identities', () => {
    const identities = getIdentitiesResponse.getIdentities();
    const proof = getIdentitiesResponse.getProof();

    expect(identities).to.deep.members([identityFixture.toBuffer()]);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getIdentitiesResponse = new GetIdentitiesByPublicKeyHashesResponseClass(
      [],
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const identities = getIdentitiesResponse.getIdentities();
    const proof = getIdentitiesResponse.getProof();

    expect(identities).to.deep.members([]);
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getRootTreeProof()).to.deep.equal(proofFixture.rootTreeProof);
    expect(proof.getStoreTreeProofs()).to.deep.equal(proofFixture.storeTreeProofs);
    expect(proof.getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should create an instance from proto', () => {
    getIdentitiesResponse = GetIdentitiesByPublicKeyHashesResponseClass.createFromProto(proto);
    expect(getIdentitiesResponse).to.be.an.instanceOf(
      GetIdentitiesByPublicKeyHashesResponseClass,
    );
    expect(getIdentitiesResponse.getIdentities()).to.deep.members([identityFixture.toBuffer()]);

    expect(getIdentitiesResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getIdentitiesResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getIdentitiesResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getIdentitiesResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();
    const storeTreeProofsProto = new StoreTreeProofs();
    storeTreeProofsProto.setIdentitiesProof(proofFixture.storeTreeProofs.identitiesProof);
    storeTreeProofsProto.setPublicKeyHashesToIdentityIdsProof(
      proofFixture.storeTreeProofs.publicKeyHashesToIdentityIdsProof,
    );
    storeTreeProofsProto.setDataContractsProof(proofFixture.storeTreeProofs.dataContractsProof);
    storeTreeProofsProto.setDocumentsProof(proofFixture.storeTreeProofs.documentsProof);
    proofProto.setSignatureLlmqHash(proofFixture.signatureLLMQHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setRootTreeProof(proofFixture.rootTreeProof);
    proofProto.setStoreTreeProofs(storeTreeProofsProto);

    proto.setIdentitiesList([]);
    proto.setProof(proofProto);

    getIdentitiesResponse = GetIdentitiesByPublicKeyHashesResponseClass.createFromProto(proto);
    expect(getIdentitiesResponse).to.be.an.instanceOf(
      GetIdentitiesByPublicKeyHashesResponseClass,
    );
    expect(getIdentitiesResponse.getIdentities()).to.deep.members([]);
    expect(getIdentitiesResponse.getMetadata()).to.deep.equal(metadataFixture);

    expect(getIdentitiesResponse.getProof())
      .to.be.an.instanceOf(Proof);
    expect(getIdentitiesResponse.getProof().getRootTreeProof())
      .to.deep.equal(proofFixture.rootTreeProof);
    expect(getIdentitiesResponse.getProof().getStoreTreeProofs())
      .to.deep.equal(proofFixture.storeTreeProofs);
    expect(getIdentitiesResponse.getProof().getSignatureLLMQHash())
      .to.deep.equal(proofFixture.signatureLLMQHash);
    expect(getIdentitiesResponse.getProof().getSignature())
      .to.deep.equal(proofFixture.signature);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.setMetadata(undefined);

    try {
      getIdentitiesResponse = GetIdentitiesByPublicKeyHashesResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
