const {
  v0: {
    GetVersionUpgradeVoteStatusResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetVersionUpgradeVoteStatusResponseClass = require('../../../../../lib/methods/platform/getVersionUpgradeVoteStatus/GetVersionUpgradeVoteStatusResponse');
const VersionSignalClass = require('../../../../../lib/methods/platform/getVersionUpgradeVoteStatus/VersionSignal');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetVersionUpgradeVoteStatusResponse', () => {
  let getVersionUpgradeVoteStatus;
  let metadataFixture;
  let versionSignalFixture;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    versionSignalFixture = new VersionSignalClass(Buffer.alloc(32).toString('hex'), 1);
    proofFixture = getProofFixture();

    const { GetVersionUpgradeVoteStatusResponseV0 } = GetVersionUpgradeVoteStatusResponse;
    const { VersionSignal, VersionSignals } = GetVersionUpgradeVoteStatusResponseV0;
    proto = new GetVersionUpgradeVoteStatusResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetVersionUpgradeVoteStatusResponseV0()
        .setVersions(new VersionSignals()
          .setVersionSignalsList([new VersionSignal()
            .setProTxHash(versionSignalFixture.getProTxHash())
            .setVersion(versionSignalFixture.getVersion()),
          ]))
        .setMetadata(metadata),
    );

    getVersionUpgradeVoteStatus = new GetVersionUpgradeVoteStatusResponseClass(
      [versionSignalFixture],
      new Metadata(metadataFixture),
    );
  });

  it('should return EpochsInfo', () => {
    const epochsInfo = getVersionUpgradeVoteStatus.getVersionSignals();
    const proof = getVersionUpgradeVoteStatus.getProof();

    expect(epochsInfo).to.deep.equal([versionSignalFixture]);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getVersionUpgradeVoteStatus = new GetVersionUpgradeVoteStatusResponseClass(
      [],
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const epochsInfo = getVersionUpgradeVoteStatus.getVersionSignals();
    const proof = getVersionUpgradeVoteStatus.getProof();

    expect(epochsInfo).to.deep.equal([]);
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should create an instance from proto', () => {
    getVersionUpgradeVoteStatus = GetVersionUpgradeVoteStatusResponseClass
      .createFromProto(proto);
    expect(getVersionUpgradeVoteStatus)
      .to.be.an.instanceOf(GetVersionUpgradeVoteStatusResponseClass);
    expect(getVersionUpgradeVoteStatus.getVersionSignals()).to.deep.equal([versionSignalFixture]);

    expect(getVersionUpgradeVoteStatus.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getVersionUpgradeVoteStatus.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getVersionUpgradeVoteStatus.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getVersionUpgradeVoteStatus.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setVersions(undefined);
    proto.getV0().setProof(proofProto);

    getVersionUpgradeVoteStatus = GetVersionUpgradeVoteStatusResponseClass.createFromProto(proto);

    expect(getVersionUpgradeVoteStatus.getVersionSignals()).to.deep.equal([]);
    expect(getVersionUpgradeVoteStatus.getMetadata()).to.deep.equal(metadataFixture);

    const proof = getVersionUpgradeVoteStatus.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getVersionUpgradeVoteStatus = GetVersionUpgradeVoteStatusResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if Epochs are not defined', () => {
    proto.getV0().setVersions(undefined);

    try {
      getVersionUpgradeVoteStatus = GetVersionUpgradeVoteStatusResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
