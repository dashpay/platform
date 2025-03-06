const {
  v0: {
    GetEpochsInfoResponse,
    ResponseMetadata,
    Proof: ProofResponse,
  },
} = require('@dashevo/dapi-grpc');

const GetEpochsInfoResponseClass = require('../../../../../lib/methods/platform/getEpochsInfo/GetEpochsInfoResponse');
const EpochInfoClass = require('../../../../../lib/methods/platform/getEpochsInfo/EpochInfo');
const getMetadataFixture = require('../../../../../lib/test/fixtures/getMetadataFixture');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const getProofFixture = require('../../../../../lib/test/fixtures/getProofFixture');
const Proof = require('../../../../../lib/methods/platform/response/Proof');
const Metadata = require('../../../../../lib/methods/platform/response/Metadata');

describe('GetEpochsInfoResponse', () => {
  let getEpochsInfoResponse;
  let metadataFixture;
  let epochInfoFixture;
  let proto;
  let proofFixture;

  beforeEach(async () => {
    metadataFixture = getMetadataFixture();
    epochInfoFixture = new EpochInfoClass(1, BigInt(1), 1, BigInt(Date.now()), 1.1);
    proofFixture = getProofFixture();

    const { GetEpochsInfoResponseV0 } = GetEpochsInfoResponse;
    const { EpochInfo, EpochInfos } = GetEpochsInfoResponseV0;
    proto = new GetEpochsInfoResponse();

    const metadata = new ResponseMetadata();
    metadata.setHeight(metadataFixture.height);
    metadata.setCoreChainLockedHeight(metadataFixture.coreChainLockedHeight);
    metadata.setTimeMs(metadataFixture.timeMs);
    metadata.setProtocolVersion(metadataFixture.protocolVersion);

    proto.setV0(
      new GetEpochsInfoResponseV0()
        .setEpochs(new EpochInfos()
          .setEpochInfosList([new EpochInfo()
            .setNumber(epochInfoFixture.getNumber())
            .setFirstBlockHeight(epochInfoFixture.getFirstBlockHeight())
            .setFirstCoreBlockHeight(epochInfoFixture.getFirstCoreBlockHeight())
            .setStartTime(epochInfoFixture.getStartTime())
            .setFeeMultiplier(epochInfoFixture.getFeeMultiplier())]))
        .setMetadata(metadata),
    );

    getEpochsInfoResponse = new GetEpochsInfoResponseClass(
      [epochInfoFixture],
      new Metadata(metadataFixture),
    );
  });

  it('should return EpochsInfo', () => {
    const epochsInfo = getEpochsInfoResponse.getEpochsInfo();
    const proof = getEpochsInfoResponse.getProof();

    expect(epochsInfo).to.deep.equal([epochInfoFixture]);
    expect(proof).to.equal(undefined);
  });

  it('should return proof', () => {
    getEpochsInfoResponse = new GetEpochsInfoResponseClass(
      [],
      new Metadata(metadataFixture),
      new Proof(proofFixture),
    );

    const epochsInfo = getEpochsInfoResponse.getEpochsInfo();
    const proof = getEpochsInfoResponse.getProof();
    const metadata = getEpochsInfoResponse.getMetadata();

    expect(epochsInfo).to.deep.equal([]);
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);

    expect(metadata.getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(metadata.getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(metadata.getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(metadata.getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);
  });

  it('should create an instance from proto', () => {
    getEpochsInfoResponse = GetEpochsInfoResponseClass.createFromProto(proto);
    expect(getEpochsInfoResponse).to.be.an.instanceOf(GetEpochsInfoResponseClass);
    expect(getEpochsInfoResponse.getEpochsInfo()).to.deep.equal([epochInfoFixture]);

    expect(getEpochsInfoResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getEpochsInfoResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getEpochsInfoResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getEpochsInfoResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    expect(getEpochsInfoResponse.getProof()).to.equal(undefined);
  });

  it('should create an instance with proof from proto', () => {
    const proofProto = new ProofResponse();

    proofProto.setQuorumHash(proofFixture.quorumHash);
    proofProto.setSignature(proofFixture.signature);
    proofProto.setGrovedbProof(proofFixture.merkleProof);
    proofProto.setRound(proofFixture.round);

    proto.getV0().setEpochs(undefined);
    proto.getV0().setProof(proofProto);

    getEpochsInfoResponse = GetEpochsInfoResponseClass.createFromProto(proto);

    expect(getEpochsInfoResponse.getEpochsInfo()).to.deep.equal([]);

    expect(getEpochsInfoResponse.getMetadata().getHeight())
      .to.deep.equal(BigInt(metadataFixture.height));
    expect(getEpochsInfoResponse.getMetadata().getCoreChainLockedHeight())
      .to.deep.equal(metadataFixture.coreChainLockedHeight);
    expect(getEpochsInfoResponse.getMetadata().getTimeMs())
      .to.deep.equal(BigInt(metadataFixture.timeMs));
    expect(getEpochsInfoResponse.getMetadata().getProtocolVersion())
      .to.deep.equal(metadataFixture.protocolVersion);

    const proof = getEpochsInfoResponse.getProof();
    expect(proof).to.be.an.instanceOf(Proof);
    expect(proof.getGrovedbProof()).to.deep.equal(proofFixture.merkleProof);
    expect(proof.getQuorumHash()).to.deep.equal(proofFixture.quorumHash);
    expect(proof.getSignature()).to.deep.equal(proofFixture.signature);
    expect(proof.getRound()).to.deep.equal(proofFixture.round);
  });

  it('should throw InvalidResponseError if Metadata is not defined', () => {
    proto.getV0().setMetadata(undefined);

    try {
      getEpochsInfoResponse = GetEpochsInfoResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });

  it('should throw InvalidResponseError if Epochs are not defined', () => {
    proto.getV0().setEpochs(undefined);

    try {
      getEpochsInfoResponse = GetEpochsInfoResponseClass.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
