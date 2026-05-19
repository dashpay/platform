import getIdentityFixture from '@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture.js';
import dapiGrpc from '@dashevo/dapi-grpc';
import GetIdentityResponseClass from '../../../../../lib/methods/platform/getIdentity/GetIdentityResponse.js';
import getMetadataFixture from '../../../../../lib/test/fixtures/getMetadataFixture.js';
import InvalidResponseError from '../../../../../lib/methods/platform/response/errors/InvalidResponseError.js';
import getProofFixture from '../../../../../lib/test/fixtures/getProofFixture.js';
import Proof from '../../../../../lib/methods/platform/response/Proof.js';
import Metadata from '../../../../../lib/methods/platform/response/Metadata.js';

const {
  v0: {
    GetIdentityResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = dapiGrpc;

describe('GetIdentityResponse', () => {
  let getIdentityResponse;
  let metadataFixture;
  let identityFixture;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    identityFixture = await getIdentityFixture();
    proofFixture = getProofFixture();

    const { GetIdentityResponseV0 } = GetIdentityResponse;
    proto = new GetIdentityResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetIdentityResponseV0()
        .setIdentity(identityFixture.toBuffer())
        .setMetadata(metadata),

    );

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
      new Uint8Array(0),
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const identity = getIdentityResponse.getIdentity();
    const proof = getIdentityResponse.getProof();

    expect(identity).to.deep.equal(new Uint8Array(0));
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should create an instance from proto', () => {
    getIdentityResponse = GetIdentityResponseClass.createFromProto(proto);
    expect(getIdentityResponse).to.be.an.instanceOf(GetIdentityResponseClass);
    expect(getIdentityResponse.getIdentity()).to.deep.equal(identityFixture.toBuffer());

    expect(getIdentityResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getIdentityResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getIdentityResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getIdentityResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(getIdentityResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setIdentity(undefined);
    proto.getV0().setProof(proofProto);

    getIdentityResponse = GetIdentityResponseClass.createFromProto(proto);

    expect(getIdentityResponse.getIdentity()).to.deep.equal(new Uint8Array(0));

    expect(getIdentityResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getIdentityResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getIdentityResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getIdentityResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    const proof = getIdentityResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getIdentityResponse = GetIdentityResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if Identity is not defined', () => {
    proto.getV0().setIdentity(undefined);

    try {
      getIdentityResponse = GetIdentityResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
