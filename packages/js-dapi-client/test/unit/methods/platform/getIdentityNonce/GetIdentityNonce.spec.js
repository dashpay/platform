const {
  v0: {
    GetIdentityNonceResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityNonceResponseClass = require('../../../../../lib/methods/platform/getIdentityNonce/GetIdentityNonceResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetIdentityNonceResponse', () => {
  let getIdentityNonceResponse;
  let metadataFixture;
  let nonce;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    nonce = BigInt(1);
    proofFixture = getProofFixture();

    const { GetIdentityNonceResponseV0 } = GetIdentityNonceResponse;
    proto = new GetIdentityNonceResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetIdentityNonceResponseV0()
        .setIdentityNonce(nonce)
        .setMetadata(metadata),
    );

    getIdentityNonceResponse = new GetIdentityNonceResponseClass(
      nonce,
      new Metadata(metadataFixture),
    );
  });

  it('should return Identity', () => {
    const IdentityNonce = getIdentityNonceResponse.getIdentityNonce();
    const proof = getIdentityNonceResponse.getProof();

    expect(IdentityNonce).to.deep.equal(nonce);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getIdentityNonceResponse = new GetIdentityNonceResponseClass(
      Buffer.alloc(0),
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const IdentityNonce = getIdentityNonceResponse.getIdentityNonce();
    const proof = getIdentityNonceResponse.getProof();

    expect(IdentityNonce).to.deep.equal(Buffer.alloc(0));
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should create an instance from proto', () => {
    getIdentityNonceResponse = GetIdentityNonceResponseClass.createFromProto(proto);
    expect(getIdentityNonceResponse).to.be
      .an.instanceOf(GetIdentityNonceResponseClass);
    expect(getIdentityNonceResponse.getIdentityNonce()).to.deep.equal(nonce);

    expect(getIdentityNonceResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getIdentityNonceResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getIdentityNonceResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getIdentityNonceResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setIdentityNonce(undefined);
    proto.getV0().setProof(proofProto);

    getIdentityNonceResponse = GetIdentityNonceResponseClass.createFromProto(proto);

    expect(getIdentityNonceResponse.getIdentityNonce())
      .to.deep.equal(BigInt(0));

    expect(getIdentityNonceResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getIdentityNonceResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getIdentityNonceResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getIdentityNonceResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    const proof = getIdentityNonceResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getIdentityNonceResponse = GetIdentityNonceResponseClass
        .createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
