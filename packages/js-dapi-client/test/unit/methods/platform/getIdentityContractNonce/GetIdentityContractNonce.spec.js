const {
  v0: {
    GetIdentityContractNonceResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityContractNonceResponseClass = require('../../../../../lib/methods/platform/getIdentityContractNonce/GetIdentityContractNonceResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetIdentityContractNonceResponse', () => {
  let getIdentityContractNonceResponse;
  let metadataFixture;
  let nonce;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    nonce = BigInt(1);
    proofFixture = getProofFixture();

    const { GetIdentityContractNonceResponseV0 } = GetIdentityContractNonceResponse;
    proto = new GetIdentityContractNonceResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetIdentityContractNonceResponseV0()
        .setIdentityContractNonce(nonce)
        .setMetadata(metadata),
    );

    getIdentityContractNonceResponse = new GetIdentityContractNonceResponseClass(
      nonce,
      new Metadata(metadataFixture),
    );
  });

  it('should return Identity', () => {
    const identityContractNonce = getIdentityContractNonceResponse.getIdentityContractNonce();
    const proof = getIdentityContractNonceResponse.getProof();

    expect(identityContractNonce).to.deep.equal(nonce);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getIdentityContractNonceResponse = new GetIdentityContractNonceResponseClass(
      Buffer.alloc(0),
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const identityContractNonce = getIdentityContractNonceResponse.getIdentityContractNonce();
    const proof = getIdentityContractNonceResponse.getProof();

    expect(identityContractNonce).to.deep.equal(Buffer.alloc(0));
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should create an instance from proto', () => {
    getIdentityContractNonceResponse = GetIdentityContractNonceResponseClass.createFromProto(proto);
    expect(getIdentityContractNonceResponse).to.be
      .an.instanceOf(GetIdentityContractNonceResponseClass);
    expect(getIdentityContractNonceResponse.getIdentityContractNonce()).to.deep.equal(nonce);

    expect(getIdentityContractNonceResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getIdentityContractNonceResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getIdentityContractNonceResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getIdentityContractNonceResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setIdentityContractNonce(undefined);
    proto.getV0().setProof(proofProto);

    getIdentityContractNonceResponse = GetIdentityContractNonceResponseClass.createFromProto(proto);

    expect(getIdentityContractNonceResponse.getIdentityContractNonce())
      .to.deep.equal(BigInt(0));

    expect(getIdentityContractNonceResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getIdentityContractNonceResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getIdentityContractNonceResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getIdentityContractNonceResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    const proof = getIdentityContractNonceResponse.getProof();

    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getIdentityContractNonceResponse = GetIdentityContractNonceResponseClass
        .createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
