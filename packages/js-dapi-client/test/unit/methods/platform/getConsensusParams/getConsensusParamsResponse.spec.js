const {
  v0: {
    GetConsensusParamsResponse: GetConsensusParamsResponseProto,
    ConsensusParamsBlock: ConsensusParamsBlockProto,
    ConsensusParamsEvidence: ConsensusParamsEvidenceProto,
  },
} = require('@dashevo/dapi-grpc');
const GetConsensusParamsResponse = require('../../../../../lib/methods/platform/getConsensusParams/getConsensusParamsResponse');
const InvalidResponseError = require('../../../../../lib/methods/platform/response/errors/InvalidResponseError');
const ConsensusParamsBlock = require('../../../../../lib/methods/platform/getConsensusParams/ConsensusParamsBlock');
const ConsensusParamsEvidence = require('../../../../../lib/methods/platform/getConsensusParams/ConsensusParamsEvidence');

describe('getConsensusParamsResponse', () => {
  let getConsensusParamsResponse;
  let consensusParamsFixture;

  beforeEach(() => {
    consensusParamsFixture = {
      block: {
        timeIotaMs: 1000,
        maxGas: -1,
        maxBytes: 22020103,
      },
      evidence: {
        maxAgeNumBlocks: 100007,
        maxAgeDuration: 172807000000007,
        maxBytes: 1048583,
      },
    };

    const block = new ConsensusParamsBlock(
      consensusParamsFixture.block.maxBytes,
      consensusParamsFixture.block.maxGas,
      consensusParamsFixture.block.timeIotaMs,
    );

    const evidence = new ConsensusParamsEvidence(
      consensusParamsFixture.evidence.maxAgeNumBlocks,
      consensusParamsFixture.evidence.maxAgeDuration,
      consensusParamsFixture.evidence.maxBytes,
    );

    getConsensusParamsResponse = new GetConsensusParamsResponse(
      block,
      evidence,
    );
  });

  it('should return block', () => {
    const block = getConsensusParamsResponse.getBlock();

    expect(block).to.be.an.instanceOf(ConsensusParamsBlock);
    expect(block.getMaxBytes()).to.deep.equal(consensusParamsFixture.block.maxBytes);
    expect(block.getMaxGas()).to.deep.equal(consensusParamsFixture.block.maxGas);
    expect(block.getTimeIotaMs()).to.deep.equal(consensusParamsFixture.block.timeIotaMs);
  });

  it('should return evidence', () => {
    const evidence = getConsensusParamsResponse.getEvidence();

    expect(evidence).to.be.an.instanceOf(ConsensusParamsEvidence);
    expect(evidence.getMaxAgeNumBlocks())
      .to.deep.equal(consensusParamsFixture.evidence.maxAgeNumBlocks);
    expect(evidence.getMaxAgeDuration())
      .to.deep.equal(consensusParamsFixture.evidence.maxAgeDuration);
    expect(evidence.getMaxBytes())
      .to.deep.equal(consensusParamsFixture.evidence.maxBytes);
  });

  it('should create an instance from proto', () => {
    const block = new ConsensusParamsBlockProto();
    block.setMaxBytes(consensusParamsFixture.block.maxBytes);
    block.setMaxGas(consensusParamsFixture.block.maxGas);
    block.setTimeIotaMs(consensusParamsFixture.block.timeIotaMs);

    const evidence = new ConsensusParamsEvidenceProto();
    evidence.setMaxAgeNumBlocks(consensusParamsFixture.evidence.maxAgeNumBlocks);
    evidence.setMaxAgeDuration(consensusParamsFixture.evidence.maxAgeDuration);
    evidence.setMaxBytes(consensusParamsFixture.evidence.maxBytes);

    const proto = new GetConsensusParamsResponseProto();
    proto.setBlock(block);
    proto.setEvidence(evidence);

    getConsensusParamsResponse = GetConsensusParamsResponse.createFromProto(proto);

    expect(getConsensusParamsResponse.getBlock()).to.be.an.instanceOf(ConsensusParamsBlock);
    expect(getConsensusParamsResponse.getBlock().getMaxBytes())
      .to.deep.equal(consensusParamsFixture.block.maxBytes);
    expect(getConsensusParamsResponse.getBlock().getMaxGas())
      .to.deep.equal(consensusParamsFixture.block.maxGas);
    expect(getConsensusParamsResponse.getBlock().getTimeIotaMs())
      .to.deep.equal(consensusParamsFixture.block.timeIotaMs);

    expect(getConsensusParamsResponse.getEvidence()).to.be.an.instanceOf(ConsensusParamsEvidence);
    expect(getConsensusParamsResponse.getEvidence().getMaxAgeNumBlocks())
      .to.deep.equal(consensusParamsFixture.evidence.maxAgeNumBlocks);
    expect(getConsensusParamsResponse.getEvidence().getMaxAgeDuration())
      .to.deep.equal(consensusParamsFixture.evidence.maxAgeDuration);
    expect(getConsensusParamsResponse.getEvidence().getMaxBytes())
      .to.deep.equal(consensusParamsFixture.evidence.maxBytes);
  });

  it('should return InvalidResponseError if consensus params are not defined', () => {
    const proto = new GetConsensusParamsResponseProto();

    try {
      getConsensusParamsResponse = GetConsensusParamsResponse.createFromProto(proto);

      expect.fail('should throw InvalidResponseError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidResponseError);
    }
  });
});
