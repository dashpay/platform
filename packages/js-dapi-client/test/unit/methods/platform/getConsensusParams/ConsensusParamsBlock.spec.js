const ConsensusParamsBlock = require('../../../../../lib/methods/platform/getConsensusParams/ConsensusParamsBlock');

describe('ConsensusParamsBlock', () => {
  let consensusParamsBlock;
  let block;

  beforeEach(() => {
    block = {
      timeIotaMs: '1000',
      maxGas: '-1',
      maxBytes: '22020103',
    };

    consensusParamsBlock = new ConsensusParamsBlock(
      block.maxBytes,
      block.maxGas,
      block.timeIotaMs,
    );
  });

  it('should return timeIotaMs', () => {
    expect(consensusParamsBlock.getTimeIotaMs()).to.equal(block.timeIotaMs);
  });

  it('should return maxGas', () => {
    expect(consensusParamsBlock.getMaxGas()).to.equal(block.maxGas);
  });

  it('should return maxBytes', () => {
    expect(consensusParamsBlock.getMaxBytes()).to.equal(block.maxBytes);
  });
});
