const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');
const {
  v0: {
    GetIdentityByPublicKeyHashResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityByPublicKeyHashResponseClass = require('../../../../../lib/methods/platform/getIdentityByPublicKeyHash/GetIdentityByPublicKeyHashResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetIdentityByPublicKeyHashResponse', () => {
  let getIdentityResponse;
  let metadataFixture;
  let identityFixture;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    identityFixture = await getIdentityFixture();
    proofFixture = getProofFixture();

    const {
      GetIdentityByPublicKeyHashResponseV0,
    } = GetIdentityByPublicKeyHashResponse;

    proto = new GetIdentityByPublicKeyHashResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetIdentityByPublicKeyHashResponseV0()
        .setIdentity(identityFixture.toBuffer())
        .setMetadata(metadata),
    );

    getIdentityResponse = new GetIdentityByPublicKeyHashResponseClass(
      identityFixture.toBuffer(),
      new Metadata(metadataFixture),
    );
  });

  it('should return identity', () => {
    const identity = getIdentityResponse.getIdentity();
    const proof = getIdentityResponse.getProof();

    expect(identity).to.deep.equal(identityFixture.toBuffer());
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getIdentityResponse = new GetIdentityByPublicKeyHashResponseClass(
      undefined,
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const identity = getIdentityResponse.getIdentity();
    const proof = getIdentityResponse.getProof();

    expect(identity).to.equal(undefined);
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
  });

  it('should create an instance from proto', () => {
    getIdentityResponse = GetIdentityByPublicKeyHashResponseClass.createFromProto(proto);
    expect(getIdentityResponse).to.be.an.instanceOf(
      GetIdentityByPublicKeyHashResponseClass,
    );
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

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setProof(proofProto);
    proto.getV0().setIdentity(undefined);

    getIdentityResponse = GetIdentityByPublicKeyHashResponseClass.createFromProto(proto);
    expect(getIdentityResponse).to.be.an.instanceOf(
      GetIdentityByPublicKeyHashResponseClass,
    );
    expect(getIdentityResponse.getIdentity()).to.deep.equal(Buffer.alloc(0));
    expect(getIdentityResponse.getMetadata()).to.deep.equal(metadataFixture);

    expect(getIdentityResponse.getProof())
      .to.be.an.instanceOf(Proof);
    expect(getIdentityResponse.getProof().getGrovedbProof())
      .to.deep.equal(proofFixture.merkleProof);
    expect(getIdentityResponse.getProof().getQuorumHash())
      .to.deep.equal(proofFixture.quorumHash);
    expect(getIdentityResponse.getProof().getSignature())
      .to.deep.equal(proofFixture.signature);
    expect(getIdentityResponse.getProof().getRound())
      .to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getIdentityResponse = GetIdentityByPublicKeyHashResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
