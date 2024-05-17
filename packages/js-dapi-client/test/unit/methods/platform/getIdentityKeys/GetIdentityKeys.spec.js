const {
  v0: {
    GetIdentityKeysResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityKeysResponseClass = require('../../../../../lib/methods/platform/getIdentityKeys/GetIdentityKeysResponse');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetIdentityKeysResponse', () => {
  let getIdentityKeysResponse;
  let metadataFixture;
  let keys;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    keys = [Buffer.alloc(41).fill(1), Buffer.alloc(48).fill(2), Buffer.alloc(55).fill(3)];
    proofFixture = getProofFixture();

    const { GetIdentityKeysResponseV0 } = GetIdentityKeysResponse;
    proto = new GetIdentityKeysResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    const { Keys } = GetIdentityKeysResponseV0;

    proto.setV0(
      new GetIdentityKeysResponseV0()
        .setKeys(new Keys().setKeysBytesList(keys))
        .setMetadata(metadata),
    );

    getIdentityKeysResponse = new GetIdentityKeysResponseClass(
      keys,
      new Metadata(metadataFixture),
    );
  });

  it('should return Identity Keys', () => {
    const identityKeys = getIdentityKeysResponse.getIdentityKeys();
    const proof = getIdentityKeysResponse.getProof();

    expect(identityKeys).to.deep.equal(keys);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getIdentityKeysResponse = new GetIdentityKeysResponseClass(
      Buffer.alloc(0),
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const identityKeys = getIdentityKeysResponse.getIdentityKeys();
    const proof = getIdentityKeysResponse.getProof();

    expect(identityKeys).to.deep.equal(Buffer.alloc(0));
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should create an instance from proto', () => {
    getIdentityKeysResponse = GetIdentityKeysResponseClass.createFromProto(proto);
    expect(getIdentityKeysResponse).to.be
      .an.instanceOf(GetIdentityKeysResponseClass);
    expect(getIdentityKeysResponse.getIdentityKeys()).to.deep.equal(keys);

    expect(getIdentityKeysResponse.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getIdentityKeysResponse.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getIdentityKeysResponse.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getIdentityKeysResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setKeys(undefined);
    proto.getV0().setProof(proofProto);

    getIdentityKeysResponse = GetIdentityKeysResponseClass.createFromProto(proto);

    expect(getIdentityKeysResponse.getIdentityKeys())
      .to.deep.equal([]);
    expect(getIdentityKeysResponse.getMetadata()).to.deep.equal(metadataFixture);

    const proof = getIdentityKeysResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getIdentityKeysResponse = GetIdentityKeysResponseClass
        .createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
