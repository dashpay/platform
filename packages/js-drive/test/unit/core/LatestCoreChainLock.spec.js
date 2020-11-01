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

    it('should emit updated chainLock', (done) => {
      const chainLock = 'someValue';
      const latestCoreChainLock = new LatestCoreChainLock();

      latestCoreChainLock.on(LatestCoreChainLock.EVENTS.update, (data) => {
        expect(data).to.equal(chainLock);

        done();
      });

      latestCoreChainLock.update(chainLock);
    });
  });

  describe('#getChainLock', () => {
    it('should return chainLock', async () => {
      const chainLock = 'someValue';

      const latestCoreChainLock = new LatestCoreChainLock();
      latestCoreChainLock.update(chainLock);

      expect(latestCoreChainLock.getChainLock()).to.equal(chainLock);
    });
  });
});
