const {
  v0: {
    GetVersionUpgradeStateResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetVersionUpgradeStateResponseClass = require('../../../../../lib/methods/platform/getProtocolVersionUpgradeState/GetProtocolVersionUpgradeStateResponse');
const VersionEntryClass = require('../../../../../lib/methods/platform/getProtocolVersionUpgradeState/VersionEntry');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetVersionUpgradeStateResponse', () => {
  let getProtocolVersionUpgradeState;
  let metadataFixture;
  let versionEntryFixture;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    versionEntryFixture = new VersionEntryClass(1, 1);
    proofFixture = getProofFixture();

    const { GetVersionUpgradeStateResponseV0 } = GetVersionUpgradeStateResponse;
    const { Versions, VersionEntry } = GetVersionUpgradeStateResponseV0;
    proto = new GetVersionUpgradeStateResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetVersionUpgradeStateResponseV0()
        .setVersions(new Versions()
          .setVersionsList([new VersionEntry()
            .setVersionNumber(versionEntryFixture.getVersionNumber())
            .setVoteCount(versionEntryFixture.getVoteCount()),
          ]))
        .setMetadata(metadata),
    );

    getProtocolVersionUpgradeState = new GetVersionUpgradeStateResponseClass(
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
    getProtocolVersionUpgradeState = new GetVersionUpgradeStateResponseClass(
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
    getProtocolVersionUpgradeState = GetVersionUpgradeStateResponseClass
      .createFromProto(proto);
    expect(getProtocolVersionUpgradeState)
      .to.be.an.instanceOf(GetVersionUpgradeStateResponseClass);
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

    getProtocolVersionUpgradeState = GetVersionUpgradeStateResponseClass.createFromProto(proto);

    expect(getProtocolVersionUpgradeState.getVersionEntries()).to.deep.equal([]);
    expect(getProtocolVersionUpgradeState.getMetadata()).to.deep.equal(metadataFixture);

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
      getProtocolVersionUpgradeState = GetVersionUpgradeStateResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if version entries are not defined', () => {
    proto.getV0().setVersions(undefined);

    try {
      getProtocolVersionUpgradeState = GetVersionUpgradeStateResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
