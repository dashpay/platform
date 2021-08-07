const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const {
  v0: {
    GetIdentityIdsByPublicKeyHashesResponse,
    ResponseMetadata,
    Proof: ProofResponse,
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityIdsByPublicKeyHashesResponseClass = require('../../../../../lib/methods/platform/getIdentityIdsByPublicKeyHashes/GetIdentityIdsByPublicKeyHashesResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetIdentityIdsByPublicKeyHashesResponse', () => {
  let getIdentityIdsByPublicKeyHashesResponse;
  let metadataFixture;
  let identityFixture;
  let proto;
  let proofFixture;

  beforeEach(() => {
    metadataFixture = getMetadataFixture();
    identityFixture = getIdentityFixture();
    proofFixture = getProofFixture();

    proto = new GetIdentityIdsByPublicKeyHashesResponse();
    proto.setIdentityIdsList(
      [identityFixture.getId()],
    );

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    getIdentityIdsByPublicKeyHashesResponse = new GetIdentityIdsByPublicKeyHashesResponseClass(
      [identityFixture.getId()],
      new Metadata(metadataFixture),
    );
  });

  it('should return Identity IDs', () => {
    const identityIds = getIdentityIdsByPublicKeyHashesResponse.getIdentityIds();
    const proof = getIdentityIdsByPublicKeyHashesResponse.getProof();

    expect(identityIds).to.deep.members([identityFixture.getId()]);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getIdentityIdsByPublicKeyHashesResponse = new GetIdentityIdsByPublicKeyHashesResponseClass(
      [],
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const identityIds = getIdentityIdsByPublicKeyHashesResponse.getIdentityIds();
    const proof = getIdentityIdsByPublicKeyHashesResponse.getProof();

    expect(identityIds).to.deep.members([]);
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getRootTreeProof()).to.deep.equal(proofFixture.rootTreeProof);
    expect(proof.getStoreTreeProofs()).to.deep.equal(proofFixture.storeTreeProofs);
    expect(proof.getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should create an instance from proto', () => {
    getIdentityIdsByPublicKeyHashesResponse = GetIdentityIdsByPublicKeyHashesResponseClass
      .createFromProto(proto);
    expect(getIdentityIdsByPublicKeyHashesResponse).to.be.an.instanceOf(
      GetIdentityIdsByPublicKeyHashesResponseClass,
    );
    expect(getIdentityIdsByPublicKeyHashesResponse.getIdentityIds())
      .to.deep.members([identityFixture.getId()]);

    expect(getIdentityIdsByPublicKeyHashesResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getIdentityIdsByPublicKeyHashesResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getIdentityIdsByPublicKeyHashesResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getIdentityIdsByPublicKeyHashesResponse.getProof()).to.equal(undefined);
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

    proto.setIdentityIdsList([]);
    proto.setProof(proofProto);

    getIdentityIdsByPublicKeyHashesResponse = GetIdentityIdsByPublicKeyHashesResponseClass
      .createFromProto(proto);

    expect(getIdentityIdsByPublicKeyHashesResponse.getIdentityIds()).to.deep.members([]);

    expect(getIdentityIdsByPublicKeyHashesResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getIdentityIdsByPublicKeyHashesResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getIdentityIdsByPublicKeyHashesResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    const proof = getIdentityIdsByPublicKeyHashesResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getRootTreeProof()).to.deep.equal(proofFixture.rootTreeProof);
    expect(proof.getStoreTreeProofs()).to.deep.equal(proofFixture.storeTreeProofs);
    expect(proof.getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.setMetadata(undefined);

    try {
      getIdentityIdsByPublicKeyHashesResponse = GetIdentityIdsByPublicKeyHashesResponseClass
        .createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
