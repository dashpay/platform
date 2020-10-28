const LatestCoreChainLock = require('../../../lib/core/LatestCoreChainLock');

describe('LatestCoreChainLock', () => {
  beforeEach(() => {
  });

  describe('#constructor', () => {
    it('should instantiate', () => {
      const latestCoreChainLock = new LatestCoreChainLock();
      expect(latestCoreChainLock.chainLock).to.equal(undefined);
      const latestCoreChainLockWithValue = new LatestCoreChainLock('someValue');
      expect(latestCoreChainLockWithValue.chainLock).to.equal('someValue');
    });
  });
  describe('#update', () => {
    it('should update', () => {
      const latestCoreChainLock = new LatestCoreChainLock();
      latestCoreChainLock.update('someValue');
      expect(latestCoreChainLock.chainLock).to.equal('someValue');
    });
  });
});
