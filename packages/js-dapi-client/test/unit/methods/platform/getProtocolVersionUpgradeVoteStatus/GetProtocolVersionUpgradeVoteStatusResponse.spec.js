const {
  v0: {
    GetProtocolVersionUpgradeVoteStatusResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetProtocolVersionUpgradeVoteStatusResponseClass = require('../../../../../lib/methods/platform/getProtocolVersionUpgradeVoteStatus/GetProtocolVersionUpgradeVoteStatusResponse');
const VersionSignalClass = require('../../../../../lib/methods/platform/getProtocolVersionUpgradeVoteStatus/VersionSignal');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetProtocolVersionUpgradeVoteStatusResponse', () => {
  let getProtocolVersionUpgradeVoteStatus;
  let metadataFixture;
  let versionSignalFixture;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    versionSignalFixture = new VersionSignalClass(Buffer.alloc(32).toString('hex'), 1);
    proofFixture = getProofFixture();

    const {
      GetProtocolVersionUpgradeVoteStatusResponseV0,
    } = GetProtocolVersionUpgradeVoteStatusResponse;
    const { VersionSignal, VersionSignals } = GetProtocolVersionUpgradeVoteStatusResponseV0;
    proto = new GetProtocolVersionUpgradeVoteStatusResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetProtocolVersionUpgradeVoteStatusResponseV0()
        .setVersions(new VersionSignals()
          .setVersionSignalsList([new VersionSignal()
            .setProTxHash(Buffer.from(versionSignalFixture.getProTxHash(), 'hex'))
            .setVersion(versionSignalFixture.getVersion()),
          ]))
        .setMetadata(metadata),
    );

    getProtocolVersionUpgradeVoteStatus = new GetProtocolVersionUpgradeVoteStatusResponseClass(
      [versionSignalFixture],
      new Metadata(metadataFixture),
    );
  });

  it('should return votes statuses', () => {
    const versionSignals = getProtocolVersionUpgradeVoteStatus.getVersionSignals();
    const proof = getProtocolVersionUpgradeVoteStatus.getProof();

    expect(versionSignals).to.deep.equal([versionSignalFixture]);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getProtocolVersionUpgradeVoteStatus = new GetProtocolVersionUpgradeVoteStatusResponseClass(
      [],
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const versionSignals = getProtocolVersionUpgradeVoteStatus.getVersionSignals();
    const proof = getProtocolVersionUpgradeVoteStatus.getProof();

    expect(versionSignals).to.deep.equal([]);
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should create an instance from proto', () => {
    getProtocolVersionUpgradeVoteStatus = GetProtocolVersionUpgradeVoteStatusResponseClass
      .createFromProto(proto);
    expect(getProtocolVersionUpgradeVoteStatus)
      .to.be.an.instanceOf(GetProtocolVersionUpgradeVoteStatusResponseClass);
    expect(getProtocolVersionUpgradeVoteStatus.getVersionSignals())
      .to.deep.equal([versionSignalFixture]);

    expect(getProtocolVersionUpgradeVoteStatus.getMetadata())
      .to.be.an.instanceOf(Metadata);
    expect(getProtocolVersionUpgradeVoteStatus.getMetadata().getHeight())
      .to.equal(metadataFixture.height);
    expect(getProtocolVersionUpgradeVoteStatus.getMetadata().getCoreChainLockedHeight())
      .to.equal(metadataFixture.coreChainLockedHeight);

    expect(getProtocolVersionUpgradeVoteStatus.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setVersions(undefined);
    proto.getV0().setProof(proofProto);

    getProtocolVersionUpgradeVoteStatus = GetProtocolVersionUpgradeVoteStatusResponseClass
      .createFromProto(proto);

    expect(getProtocolVersionUpgradeVoteStatus.getVersionSignals()).to.deep.equal([]);
    expect(getProtocolVersionUpgradeVoteStatus.getMetadata()).to.deep.equal(metadataFixture);

    const proof = getProtocolVersionUpgradeVoteStatus.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getProtocolVersionUpgradeVoteStatus = GetProtocolVersionUpgradeVoteStatusResponseClass
        .createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if version statuses are not defined', () => {
    proto.getV0().setVersions(undefined);

    try {
      getProtocolVersionUpgradeVoteStatus = GetProtocolVersionUpgradeVoteStatusResponseClass
        .createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
