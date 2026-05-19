import dapiGrpc from '@dashevo/dapi-grpc';
import GetIdentityNonceResponseClass from '../../../../../lib/methods/platform/getIdentityNonce/GetIdentityNonceResponse.js';
import getMetadataFixture from '../../../../../lib/test/fixtures/getMetadataFixture.js';
import InvalidResponseError from '../../../../../lib/methods/platform/response/errors/InvalidResponseError.js';
import getProofFixture from '../../../../../lib/test/fixtures/getProofFixture.js';
import Proof from '../../../../../lib/methods/platform/response/Proof.js';
import Metadata from '../../../../../lib/methods/platform/response/Metadata.js';

const {
  v0: {
    GetIdentityNonceResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = dapiGrpc;

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
      new Uint8Array(0),
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const IdentityNonce = getIdentityNonceResponse.getIdentityNonce();
    const proof = getIdentityNonceResponse.getProof();

    expect(IdentityNonce).to.deep.equal(new Uint8Array(0));
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
