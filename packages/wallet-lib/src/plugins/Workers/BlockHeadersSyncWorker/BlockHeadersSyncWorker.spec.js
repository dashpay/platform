const BlockHeadersSyncWorker = require("./BlockHeadersSyncWorker");

describe("BlockHeadersSyncWorker", function suite() {
  let blockHeadersSyncWorker;

  const createBlockHeadersSyncWorker = (sinon) => {
    const blockHeadersSyncWorker = new BlockHeadersSyncWorker({
      executeOnStart: false
    });

    blockHeadersSyncWorker.network = "testnet";
    blockHeadersSyncWorker.transport = {
      client: {
        blockHeadersProvider: {
          on: sinon.stub(),
          once: sinon.stub(),
          readHistorical: sinon.stub(),
        }
      },
    }

    blockHeadersSyncWorker.storage = {
      application: {},
      getDefaultChainStore: function () {
        if (!this.defaultChainStore) {
          this.defaultChainStore = {
            state: {
              blockHeight: 0,
            }
          }
        }
        return this.defaultChainStore;
      }
    }

    return blockHeadersSyncWorker;
  }



  describe('#getStartBlockHeight', function () {
    beforeEach(function () {
      blockHeadersSyncWorker = createBlockHeadersSyncWorker(this.sinon);
    });

    it('should return block 1', () => {
      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1);
    })

    it('should return best block in case `skipSynchronization` option is present', () => {
      /**
       * Mock options
       */
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.blockHeight = 1000;

      storage.application.syncOptions = {
        skipSynchronization: true,
      }

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1000);
    })

    it('should return last synced header height if present', () => {
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.lastSyncedHeaderHeight = 1200;

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1200);
    })

    it('should return `skipSynchronizationBeforeHeight` value', () => {
      const { storage } = blockHeadersSyncWorker;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1300,
      }

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1300);
    });

    it('should return last synced header if it\'s greater than `skipSynchronizationBeforeHeight` value', () => {
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.lastSyncedHeaderHeight = 1300;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1200,
      }

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1300);
    });

    it('should return `skipSynchronizationBeforeHeight` value if it\'s greater than last synced header height', () => {
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.lastSyncedHeaderHeight = 1200;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1300,
      }

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1300);
    });
  });
});
