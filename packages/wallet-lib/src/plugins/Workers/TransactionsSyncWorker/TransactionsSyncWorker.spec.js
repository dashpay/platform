const { expect } = require('chai');
const TransactionsSyncWorker = require('./TransactionsSyncWorker');

describe('TransactionsSyncWorker', () => {
  let transactionsSyncWorker;

  const CHAIN_HEIGHT = 1000;

  const createTransactionsSyncWorker = (sinon) => {
    const worker = new TransactionsSyncWorker({
      executeOnStart: false,
    });

    worker.storage = {
      application: {},
      scheduleStateSave: sinon.spy(),
      saveState: () => {},
      getDefaultChainStore() {
        if (!this.defaultChainStore) {
          this.defaultChainStore = {
            state: {
              chainHeight: CHAIN_HEIGHT,
              lastSyncedBlockHeight: -1,
            },
            // updateLastSyncedHeaderHeight: sinon.spy(),
            // updateLastSyncedBlockHeight: sinon.spy(),
            // updateChainHeight: sinon.spy(),
            // setBlockHeaders: sinon.spy(),
            // updateHeadersMetadata: sinon.spy(),
          };
        }
        return this.defaultChainStore;
      },
    };

    return worker;
  };

  describe('#getStartBlockHeight', () => {
    beforeEach(function beforeEach() {
      transactionsSyncWorker = createTransactionsSyncWorker(this.sinon);
    });

    it('should return block 1 by default', () => {
      const startBlockHeight = transactionsSyncWorker.getStartBlockHeight();
      expect(startBlockHeight).to.equal(1);
    });

    it('should return last synced block height if present', () => {
      const { storage } = transactionsSyncWorker;
      storage.getDefaultChainStore().state.lastSyncedBlockHeight = 1200;

      const startBlockHeight = transactionsSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1200);
    });

    it('should return `skipSynchronizationBeforeHeight` value', () => {
      const { storage } = transactionsSyncWorker;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1300,
      };

      const startBlockHeight = transactionsSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1300);
    });

    it('should return `skipSynchronizationBeforeHeight` value if it\'s greater than last synced header height', () => {
      const { storage } = transactionsSyncWorker;
      storage.getDefaultChainStore().state.lastSyncedHeaderHeight = 1200;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1300,
      };

      const startBlockHeight = transactionsSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1300);
    });
  });
});
