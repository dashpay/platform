/* eslint-disable no-unused-expressions */

const EventEmitter = require('events');
const DAPIClient = require('@dashevo/dapi-client');
const { expect } = require('chai');

const { BlockHeadersProvider } = DAPIClient;

const BlockHeadersSyncWorker = require('./BlockHeadersSyncWorker');

const { waitOneTick } = require('../../../test/utils');

describe('BlockHeadersSyncWorker', () => {
  let blockHeadersSyncWorker;
  const chainHeight = 1000;

  const createBlockHeadersSyncWorker = (sinon) => {
    const worker = new BlockHeadersSyncWorker({
      executeOnStart: false,
    });

    const blockHeadersProvider = new EventEmitter();
    blockHeadersProvider.readHistorical = sinon.stub();
    blockHeadersProvider.startContinuousSync = sinon.stub();
    blockHeadersProvider.stop = () => {
      blockHeadersProvider.emit(BlockHeadersProvider.EVENTS.STOPPED);
    };
    blockHeadersProvider.spvChain = {
      getLongestChain: sinon.stub().returns([]),
      orphanChunks: [],
      prunedHeaders: [],
    };
    sinon.spy(blockHeadersProvider, 'on');
    sinon.spy(blockHeadersProvider, 'once');
    sinon.spy(blockHeadersProvider, 'removeListener');

    worker.network = 'testnet';
    worker.transport = {
      client: {
        blockHeadersProvider,
      },
    };

    worker.storage = {
      application: {},
      getDefaultChainStore() {
        if (!this.defaultChainStore) {
          this.defaultChainStore = {
            state: {
              blockHeight: chainHeight,
            },
          };
        }
        return this.defaultChainStore;
      },
    };

    return worker;
  };

  describe('#getStartBlockHeight', () => {
    beforeEach(function beforeEach() {
      blockHeadersSyncWorker = createBlockHeadersSyncWorker(this.sinon);
    });

    it('should return block 1', () => {
      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1);
    });

    it('should return bestBlockHeight - N in case `skipSynchronization` option is present', () => {
      /**
       * Mock options
       */
      blockHeadersSyncWorker.maxHeadersToKeep = 2000;
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.blockHeight = 1000;

      storage.application.syncOptions = {
        skipSynchronization: true,
      };

      let startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1);

      storage.getDefaultChainStore().state.blockHeight = 3000;
      startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();
      expect(startBlockHeight).to.equal(1000);
    });

    it('should return last synced header height if present', () => {
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.lastSyncedHeaderHeight = 1200;

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1200);
    });

    it('should return `skipSynchronizationBeforeHeight` value', () => {
      const { storage } = blockHeadersSyncWorker;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1300,
      };

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1300);
    });

    it('should return last synced header if it\'s greater than `skipSynchronizationBeforeHeight` value', () => {
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.lastSyncedHeaderHeight = 1300;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1200,
      };

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1300);
    });

    it('should return `skipSynchronizationBeforeHeight` value if it\'s greater than last synced header height', () => {
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.lastSyncedHeaderHeight = 1200;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1300,
      };

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1300);
    });
  });

  describe('#onStart', () => {
    beforeEach(function beforeEach() {
      blockHeadersSyncWorker = createBlockHeadersSyncWorker(this.sinon);
    });

    it('should kickstart reading of historical headers', async () => {
      const startPromise = blockHeadersSyncWorker.onStart();
      await waitOneTick();

      const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;

      expect(blockHeadersSyncWorker.state).to
        .equal(BlockHeadersSyncWorker.STATES.HISTORICAL_SYNC);
      expect(blockHeadersProvider.on).to
        .have.been.calledWith(
          BlockHeadersProvider.EVENTS.CHAIN_UPDATED,
          blockHeadersSyncWorker.historicalChainUpdateHandler,
        );
      expect(blockHeadersProvider.on).to
        .have.been.calledWith(
          BlockHeadersProvider.EVENTS.ERROR,
          blockHeadersSyncWorker.blockHeadersProviderErrorHandler,
        );
      expect(blockHeadersProvider.once).to
        .have.been.calledWith(
          BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED,
          blockHeadersSyncWorker.historicalDataObtainedHandler,
        );
      expect(blockHeadersProvider.once).to
        .have.been.calledWith(
          BlockHeadersProvider.EVENTS.STOPPED,
          blockHeadersSyncWorker.blockHeadersProviderStopHandler,
        );
      expect(blockHeadersProvider.readHistorical).to.have.been.calledOnceWith(1, chainHeight);

      blockHeadersProvider.emit(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);
      await startPromise;
    });

    it('should prepare for continuous sync after historical data is obtained', async function test() {
      const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;

      blockHeadersSyncWorker.updateProgress = this.sinon.spy();

      const startPromise = blockHeadersSyncWorker.onStart();
      blockHeadersProvider.emit(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);
      await startPromise;

      expect(blockHeadersSyncWorker.state).to
        .equal(BlockHeadersSyncWorker.STATES.IDLE);
      expect(blockHeadersProvider.removeListener)
        .to.have.been.calledWith(
          BlockHeadersProvider.EVENTS.CHAIN_UPDATED,
          blockHeadersSyncWorker.historicalChainUpdateHandler,
        );
      expect(blockHeadersProvider.removeListener)
        .to.have.been.calledWith(
          BlockHeadersProvider.EVENTS.ERROR,
          blockHeadersSyncWorker.blockHeadersProviderErrorHandler,
        );
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

  describe('#execute', () => {
    beforeEach(function beforeEach() {
      blockHeadersSyncWorker = createBlockHeadersSyncWorker(this.sinon);
    });

    it('should kickstart continuous sync', async () => {
      const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;

      await blockHeadersSyncWorker.execute();

      expect(blockHeadersSyncWorker.state).to
        .equal(BlockHeadersSyncWorker.STATES.CONTINUOUS_SYNC);

      expect(blockHeadersProvider.on).to
        .have.been.calledWith(
          BlockHeadersProvider.EVENTS.CHAIN_UPDATED,
          blockHeadersSyncWorker.continuousChainUpdateHandler,
        );
      expect(blockHeadersProvider.on).to
        .have.been.calledWith(
          BlockHeadersProvider.EVENTS.ERROR,
          blockHeadersSyncWorker.blockHeadersProviderErrorHandler,
        );
      expect(blockHeadersProvider.startContinuousSync).to
        .have.been.calledWith(blockHeadersSyncWorker.syncCheckpoint);
    });

    // TODO: should throw an error if sync checkpoint is not match to best block height
  });

  describe('#onStop', () => {
    beforeEach(function beforeEach() {
      blockHeadersSyncWorker = createBlockHeadersSyncWorker(this.sinon);
    });

    it('should stop historical sync', async () => {
      const promise = blockHeadersSyncWorker.onStart();
      await waitOneTick();

      await blockHeadersSyncWorker.onStop();
      await promise;

      const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;

      expect(blockHeadersProvider.removeListener).to
        .have.been.calledWith(
          BlockHeadersProvider.EVENTS.CHAIN_UPDATED,
          blockHeadersSyncWorker.historicalChainUpdateHandler,
        );
      expect(blockHeadersProvider.removeListener).to
        .have.been.calledWith(
          BlockHeadersProvider.EVENTS.ERROR,
          blockHeadersSyncWorker.blockHeadersProviderErrorHandler,
        );
      expect(blockHeadersProvider.removeListener).to
        .have.been.calledWith(
          BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED,
          blockHeadersSyncWorker.historicalDataObtainedHandler,
        );

      expect(blockHeadersSyncWorker.state).to.equal(BlockHeadersSyncWorker.STATES.IDLE);
      // TODO: make sure that syncCheckpoint is set to latest known header, not best block height
    });

    it('should stop continuous sync', async () => {
      await blockHeadersSyncWorker.execute();
      await blockHeadersSyncWorker.onStop();
      //
      const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;

      expect(blockHeadersProvider.removeListener).to
        .have.been.calledWith(
          BlockHeadersProvider.EVENTS.CHAIN_UPDATED,
          blockHeadersSyncWorker.continuousChainUpdateHandler,
        );
      expect(blockHeadersProvider.removeListener).to
        .have.been.calledWith(
          BlockHeadersProvider.EVENTS.ERROR,
          blockHeadersSyncWorker.blockHeadersProviderErrorHandler,
        );

      expect(blockHeadersSyncWorker.state).to.equal(BlockHeadersSyncWorker.STATES.IDLE);
    });

    it('should continue historical sync from checkpoint after the restart', async () => {
      let startPromise = blockHeadersSyncWorker.onStart();
      await waitOneTick();
      await blockHeadersSyncWorker.onStop();
      await startPromise;

      // TODO: make sure that syncCheckpoint is set to latest
      //  confirmed header, not best block height
      blockHeadersSyncWorker.syncCheckpoint = 980;

      startPromise = blockHeadersSyncWorker.onStart();

      const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;

      expect(blockHeadersProvider.readHistorical).to.have.been.calledTwice;
      expect(blockHeadersProvider.readHistorical.secondCall)
        .to.have.been.calledWith(980, 1000);

      blockHeadersProvider.emit(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);
      await startPromise;
    });
  });
});
