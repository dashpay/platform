const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const {
  v0: {
    GetIdentityResponse,
    ResponseMetadata,
    Proof: ProofResponse,
    StoreTreeProofs,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityResponseClass = require('../../../../../lib/methods/platform/getIdentity/GetIdentityResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetIdentityResponse', () => {
  let getIdentityResponse;
  let metadataFixture;
  let identityFixture;
  let proto;
  let proofFixture;

  beforeEach(() => {
    metadataFixture = getMetadataFixture();
    identityFixture = getIdentityFixture();
    proofFixture = getProofFixture();

    proto = new GetIdentityResponse();
    proto.setIdentity(identityFixture.toBuffer());

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);

    proto.setMetadata(metadata);

    getIdentityResponse = new GetIdentityResponseClass(
      identityFixture.toBuffer(),
      new Metadata(metadataFixture),
    );
  });

  it('should return Identity', () => {
    const identity = getIdentityResponse.getIdentity();
    const proof = getIdentityResponse.getProof();

    expect(identity).to.deep.equal(identityFixture.toBuffer());
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getIdentityResponse = new GetIdentityResponseClass(
      Buffer.alloc(0),
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const identity = getIdentityResponse.getIdentity();
    const proof = getIdentityResponse.getProof();

    expect(identity).to.deep.equal(Buffer.alloc(0));
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getRootTreeProof()).to.deep.equal(proofFixture.rootTreeProof);
    expect(proof.getStoreTreeProofs()).to.deep.equal(proofFixture.storeTreeProofs);
    expect(proof.getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should create an instance from proto', () => {
    getIdentityResponse = GetIdentityResponseClass.createFromProto(proto);
    expect(getIdentityResponse).to.be.an.instanceOf(GetIdentityResponseClass);
    expect(getIdentityResponse.getIdentity()).to.deep.equal(identityFixture.toBuffer());

    expect(getIdentityResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getIdentityResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getIdentityResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getIdentityResponse.getProof()).to.equal(undefined);
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

    proto.setIdentity(undefined);
    proto.setProof(proofProto);

    getIdentityResponse = GetIdentityResponseClass.createFromProto(proto);

    expect(getIdentityResponse.getIdentity()).to.deep.equal(Buffer.alloc(0));
    expect(getIdentityResponse.getMetadata()).to.deep.equal(metadataFixture);

    const proof = getIdentityResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getRootTreeProof()).to.deep.equal(proofFixture.rootTreeProof);
    expect(proof.getStoreTreeProofs()).to.deep.equal(proofFixture.storeTreeProofs);
    expect(proof.getSignatureLLMQHash()).to.deep.equal(proofFixture.signatureLLMQHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.setMetadata(undefined);

    try {
      getIdentityResponse = GetIdentityResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if Identity is not defined', () => {
    proto.setIdentity(undefined);

    try {
      getIdentityResponse = GetIdentityResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
