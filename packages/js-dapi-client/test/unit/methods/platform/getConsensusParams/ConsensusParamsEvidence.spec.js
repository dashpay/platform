const ConsensusParamsEvidence = require('../../../../../lib/methods/platform/getConsensusParams/ConsensusParamsEvidence');

describe('ConsensusParamsEvidence', () => {
  let consensusParamsEvidence;
  let evidence;

  beforeEach(() => {
    evidence = {
      maxAgeNumBlocks: '100007',
      maxAgeDuration: '172807000000007',
      maxBytes: '1048583',
    };

    consensusParamsEvidence = new ConsensusParamsEvidence(
      evidence.maxAgeNumBlocks,
      evidence.maxAgeDuration,
      evidence.maxBytes,
    );
  });

  it('should return maxAgeNumBlocks', () => {
    expect(consensusParamsEvidence.getMaxAgeNumBlocks()).to.equal(evidence.maxAgeNumBlocks);
  });

  it('should return maxAgeDuration', () => {
    expect(consensusParamsEvidence.getMaxAgeDuration()).to.equal(evidence.maxAgeDuration);
  });

  it('should return maxBytes', () => {
    expect(consensusParamsEvidence.getMaxBytes()).to.equal(evidence.maxBytes);
  });
});
