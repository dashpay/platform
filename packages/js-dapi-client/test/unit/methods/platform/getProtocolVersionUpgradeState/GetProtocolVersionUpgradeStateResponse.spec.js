const {
  v0: {
    GetProtocolVersionUpgradeStateResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetProtocolVersionUpgradeStateResponseClass = require('../../../../../lib/methods/platform/getProtocolVersionUpgradeState/GetProtocolVersionUpgradeStateResponse');
const VersionEntryClass = require('../../../../../lib/methods/platform/getProtocolVersionUpgradeState/VersionEntry');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetProtocolVersionUpgradeStateResponse', () => {
  let getProtocolVersionUpgradeState;
  let metadataFixture;
  let versionEntryFixture;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    versionEntryFixture = new VersionEntryClass(1, 1);
    proofFixture = getProofFixture();

    const { GetProtocolVersionUpgradeStateResponseV0 } = GetProtocolVersionUpgradeStateResponse;
    const { Versions, VersionEntry } = GetProtocolVersionUpgradeStateResponseV0;
    proto = new GetProtocolVersionUpgradeStateResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetProtocolVersionUpgradeStateResponseV0()
        .setVersions(new Versions()
          .setVersionsList([new VersionEntry()
            .setVersionNumber(versionEntryFixture.getVersionNumber())
            .setVoteCount(versionEntryFixture.getVoteCount()),
          ]))
        .setMetadata(metadata),
    );

    getProtocolVersionUpgradeState = new GetProtocolVersionUpgradeStateResponseClass(
      [versionEntryFixture],
      new Metadata(metadataFixture),
    );
  });

  it('should return version upgrade state', () => {
    const versions = getProtocolVersionUpgradeState.getVersionEntries();
    const proof = getProtocolVersionUpgradeState.getProof();

    expect(versions).to.deep.equal([versionEntryFixture]);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getProtocolVersionUpgradeState = new GetProtocolVersionUpgradeStateResponseClass(
      [],
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const versions = getProtocolVersionUpgradeState.getVersionEntries();
    const proof = getProtocolVersionUpgradeState.getProof();

    expect(versions).to.deep.equal([]);
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should create an instance from proto', () => {
    getProtocolVersionUpgradeState = GetProtocolVersionUpgradeStateResponseClass
      .createFromProto(proto);
    expect(getProtocolVersionUpgradeState)
      .to.be.an.instanceOf(GetProtocolVersionUpgradeStateResponseClass);
    expect(getProtocolVersionUpgradeState.getVersionEntries()).to.deep.equal([versionEntryFixture]);

    expect(getProtocolVersionUpgradeState.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getProtocolVersionUpgradeState.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getProtocolVersionUpgradeState.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getProtocolVersionUpgradeState.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setVersions(undefined);
    proto.getV0().setProof(proofProto);

    getProtocolVersionUpgradeState = GetProtocolVersionUpgradeStateResponseClass
      .createFromProto(proto);

    expect(getProtocolVersionUpgradeState.getVersionEntries()).to.deep.equal([]);

    expect(getProtocolVersionUpgradeState.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getProtocolVersionUpgradeState.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getProtocolVersionUpgradeState.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getProtocolVersionUpgradeState.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    const proof = getProtocolVersionUpgradeState.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getProtocolVersionUpgradeState = GetProtocolVersionUpgradeStateResponseClass
        .createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if version entries are not defined', () => {
    proto.getV0().setVersions(undefined);

    try {
      getProtocolVersionUpgradeState = GetProtocolVersionUpgradeStateResponseClass
        .createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
