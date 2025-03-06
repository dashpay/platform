const { default: loadWasmDpp } = require('../..');

let Metadata;

describe('Metadata', () => {
  beforeEach(async () => {
    ({ Metadata } = await loadWasmDpp());
  });

  describe('#constructor', () => {
    it('should set height and core chain-locked height', () => {
      const result = new Metadata(BigInt(42), 1, BigInt(100), 2);

      expect(result.getBlockHeight()).to.equal(BigInt(42));
      expect(result.getCoreChainLockedHeight()).to.equal(1);
      expect(result.getTimeMs()).to.equal(BigInt(100));
      expect(result.getProtocolVersion()).to.equal(2);
    });
  });

  describe('#getBlockHeight', () => {
    it('should get block height', () => {
      const result = new Metadata(BigInt(42), 1, BigInt(100), 2);

      expect(result.getBlockHeight()).to.equal(BigInt(42));
    });
  });

  describe('#getCoreChainLockedHeight', () => {
    it('should get core chain-locked height', () => {
      const result = new Metadata(BigInt(1), 42, BigInt(100), 2);

      expect(result.getCoreChainLockedHeight()).to.equal(42);
    });
  });
});
