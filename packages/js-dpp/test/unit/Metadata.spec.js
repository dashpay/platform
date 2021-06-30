const Metadata = require('../../lib/Metadata');

describe('Metadata', () => {
  describe('#constructor', () => {
    it('should set height and core chain-locked height', () => {
      const result = new Metadata({
        blockHeight: 42,
        coreChainLockedHeight: 1,
      });

      expect(result.blockHeight).to.equal(42);
      expect(result.coreChainLockedHeight).to.equal(1);
    });
  });

  describe('#getBlockHeight', () => {
    it('should get block height', () => {
      const result = new Metadata({
        blockHeight: 42,
        coreChainLockedHeight: 1,
      });

      expect(result.getBlockHeight()).to.equal(42);
    });
  });

  describe('#getCoreChainLockedHeight', () => {
    it('should get core chain-locked height', () => {
      const result = new Metadata({
        blockHeight: 1,
        coreChainLockedHeight: 42,
      });

      expect(result.getCoreChainLockedHeight()).to.equal(42);
    });
  });
});
