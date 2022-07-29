const DAPIClient = require('@dashevo/dapi-client');
const { expect } = require('chai');

const { BlockHeadersProvider } = DAPIClient;

const BlockHeadersSyncWorker = require("./BlockHeadersSyncWorker");

describe("BlockHeadersSyncWorker", function suite() {
  let blockHeadersSyncWorker;
  let chainHeight = 1000;

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
          spvChain: {
            getLongestChain: sinon.stub().returns([]),
            orphanChunks: [],
            prunedHeaders: []
          }
        }
      },
    }

    blockHeadersSyncWorker.storage = {
      application: {},
      getDefaultChainStore: function () {
        if (!this.defaultChainStore) {
          this.defaultChainStore = {
            state: {
              blockHeight: chainHeight,
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

    it('should return bestBlockHeight - N in case `skipSynchronization` option is present', () => {
      /**
       * Mock options
       */
      blockHeadersSyncWorker.maxHeadersToKeep = 2000;
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.blockHeight = 1000;

      storage.application.syncOptions = {
        skipSynchronization: true,
      }

      let startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1);

      storage.getDefaultChainStore().state.blockHeight = 3000;
      startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();
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

  describe('#onStart', function () {
    beforeEach(function () {
      blockHeadersSyncWorker = createBlockHeadersSyncWorker(this.sinon);
    });

    it('should kickstart reading of historical headers',  (done) => {
      blockHeadersSyncWorker.onStart().catch(done);

      const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;

      expect(blockHeadersProvider.on).to
        .have.been.calledWith(BlockHeadersProvider.EVENTS.CHAIN_UPDATED);
      expect(blockHeadersProvider.on).to
        .have.been.calledWith(BlockHeadersProvider.EVENTS.ERROR);
      expect(blockHeadersProvider.once).to
        .have.been.calledWith(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);
      expect(blockHeadersProvider.readHistorical).to.have.been.calledOnceWith(1, chainHeight)

      done();
    });

    it('should prepare for continuous sync after historical data is obtained', async function() {
      blockHeadersSyncWorker.updateProgress = this.sinon.spy();
      blockHeadersSyncWorker.createHistoricalSyncCompleteListener = () => Promise.resolve();
      await blockHeadersSyncWorker.onStart()
      expect(blockHeadersSyncWorker.syncCheckpoint).to.equal(chainHeight);
      expect(blockHeadersSyncWorker.updateProgress).to.have.been.calledOnce;
    });

    it('should throw error if best block height is less than 1', async () => {
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.blockHeight = -1;

      await expect(blockHeadersSyncWorker.onStart())
        .to.be.rejectedWith('Invalid best block height -1');
    });

    it('should throw error if start block height is greater than best block height', async () => {
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.lastSyncedHeaderHeight = 2000;

      await expect(blockHeadersSyncWorker.onStart())
        .to.be.rejectedWith('Start block height 2000 is greater than best block height 1000');
    });
  });
});
