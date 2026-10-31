/* eslint-disable no-unused-expressions */

const EventEmitter = require('events');
const DAPIClient = require('@dashevo/dapi-client');
const { Block } = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const logger = require('../../../logger');

const { BlockHeadersProvider } = DAPIClient;

const BlockHeadersSyncWorker = require('./BlockHeadersSyncWorker');

const { waitOneTick } = require('../../../test/utils');

const EVENTS = require('../../../EVENTS');
const { mockHeadersChain } = require('../../../test/mocks/dashcore/block');

describe('BlockHeadersSyncWorker', () => {
  let blockHeadersSyncWorker;
  const chainHeight = 1000;
  const spvChainHeaders = mockHeadersChain('testnet', 5);

  const createBlockHeadersSyncWorker = (sinon) => {
    const worker = new BlockHeadersSyncWorker({
      executeOnStart: false,
      maxHeadersToKeep: 3,
    });
    worker.logger = logger;

    const blockHeadersProvider = new EventEmitter();
    blockHeadersProvider.readHistorical = sinon.spy();
    blockHeadersProvider.startContinuousSync = sinon.spy();
    blockHeadersProvider.stop = () => {
      blockHeadersProvider.emit(BlockHeadersProvider.EVENTS.STOPPED);
    };
    blockHeadersProvider.spvChain = {
      startBlockHeight: 0,
      getLongestChain() {
        if (!this.longestChain) {
          this.longestChain = [...spvChainHeaders];
        }

        return this.longestChain;
      },
      getTipHeight() {
        return this.startBlockHeight + this.getLongestChain().length - 1;
      },
      getTipHash() {
        return this.getLongestChain()[this.getLongestChain().length - 1].hash;
      },
      getHeaderAt(height) {
        return this.getLongestChain()[height - this.startBlockHeight];
      },
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
      getBlockByHeight() {
        return Block.fromObject({
          header: {
            prevHash: Buffer.alloc(32),
            time: 99999999,
            merkleRoot: Buffer.alloc(32),
          },
          transactions: [],
        });
      },
    };
    worker.parentEvents = new EventEmitter();
    sinon.spy(worker.parentEvents, 'emit');
    sinon.spy(worker.transport, 'getBlockByHeight');
    sinon.spy(worker, 'scheduleProgressUpdate');

    worker.storage = {
      application: {},
      scheduleStateSave: sinon.spy(),
      saveState: () => {},
      getDefaultChainStore() {
        if (!this.defaultChainStore) {
          this.defaultChainStore = {
            state: {
              chainHeight,
              lastSyncedHeaderHeight: -1,
              blockHeaders: [],
              headersMetadata: new Map(),
              hashesByHeight: new Map(),
            },
            updateLastSyncedHeaderHeight: sinon.spy(),
            updateChainHeight: sinon.spy(),
            setBlockHeaders: sinon.spy(),
            updateHeadersMetadata: sinon.spy(),
          };
          const { state } = this.defaultChainStore;
          this.defaultChainStore.rewindHeaders = sinon.spy((height) => {
            state.lastSyncedHeaderHeight = height - 1;
          });
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

    it('should start at genesis for a new wallet without resumable headers', () => {
      /**
       * Mock options
       */
      blockHeadersSyncWorker.maxHeadersToKeep = 2000;
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.chainHeight = 1000;

      storage.application.syncOptions = {
        skipSynchronization: true,
      };

      let startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1);

      storage.getDefaultChainStore().state.chainHeight = 3000;
      startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();
      expect(startBlockHeight).to.equal(1);
    });

    it('should start at genesis for one stored header with a stale positive height', () => {
      const { storage } = blockHeadersSyncWorker;
      const { state } = storage.getDefaultChainStore();
      state.blockHeaders = [spvChainHeaders[0]];
      state.lastSyncedHeaderHeight = 1200;

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1);
    });

    it('should start at genesis for no stored headers with a stale positive height', () => {
      const { state } = blockHeadersSyncWorker.storage.getDefaultChainStore();
      state.lastSyncedHeaderHeight = 1200;

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1);
    });

    it('should return last synced header height for a resumable context', () => {
      const { storage } = blockHeadersSyncWorker;
      const { state } = storage.getDefaultChainStore();
      state.blockHeaders = spvChainHeaders.slice(0, 2);
      state.lastSyncedHeaderHeight = 1200;

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1200);
    });

    it('should start at genesis instead of `skipSynchronizationBeforeHeight`', () => {
      const { storage } = blockHeadersSyncWorker;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1300,
      };

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1);
    });

    it('should return resumable header height when it is greater than the unsafe skip', () => {
      const { storage } = blockHeadersSyncWorker;
      const { state } = storage.getDefaultChainStore();
      state.blockHeaders = spvChainHeaders.slice(0, 2);
      state.lastSyncedHeaderHeight = 1300;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1200,
      };

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1300);
    });

    it('should return resumable header height when the unsafe skip is greater', () => {
      const { storage } = blockHeadersSyncWorker;
      const { state } = storage.getDefaultChainStore();
      state.blockHeaders = spvChainHeaders.slice(0, 2);
      state.lastSyncedHeaderHeight = 1200;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1300,
      };

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1200);
    });

    it('should start at genesis when the implied first header height is negative', () => {
      const { state } = blockHeadersSyncWorker.storage.getDefaultChainStore();
      state.blockHeaders = spvChainHeaders.slice(0, 2);
      state.lastSyncedHeaderHeight = 0;

      const startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1);
    });

    it('should reject invalid persisted header state types and ranges', () => {
      const { state } = blockHeadersSyncWorker.storage.getDefaultChainStore();

      state.blockHeaders = {};
      expect(() => blockHeadersSyncWorker.getStartBlockHeight())
        .to.throw('Invalid block headers');

      state.blockHeaders = [];
      state.lastSyncedHeaderHeight = Number.MAX_SAFE_INTEGER + 1;
      expect(() => blockHeadersSyncWorker.getStartBlockHeight())
        .to.throw('Invalid last synced header height');

      state.lastSyncedHeaderHeight = -2;
      expect(() => blockHeadersSyncWorker.getStartBlockHeight())
        .to.throw('Invalid last synced header height');
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

      expect(blockHeadersSyncWorker.syncState).to
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

      expect(blockHeadersSyncWorker.syncState).to
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
      expect(blockHeadersSyncWorker.syncCheckpoint)
        .to.equal(blockHeadersProvider.spvChain.getTipHeight());
      expect(blockHeadersSyncWorker.updateProgress).to.have.been.calledOnce;
    });

    it('should throw error if best block height is less than 1', async () => {
      const { storage } = blockHeadersSyncWorker;
      storage.getDefaultChainStore().state.chainHeight = -1;

      await expect(blockHeadersSyncWorker.onStart())
        .to.be.rejectedWith('Invalid best block height -1');
    });

    it('should throw error if start block height is greater than best block height', async () => {
      const { storage } = blockHeadersSyncWorker;
      const { state } = storage.getDefaultChainStore();
      state.blockHeaders = spvChainHeaders.slice(0, 2);
      state.lastSyncedHeaderHeight = 2000;

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

      blockHeadersSyncWorker.syncCheckpoint = 1000;
      await blockHeadersSyncWorker.execute();

      expect(blockHeadersSyncWorker.syncState).to
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

    it('should forward an error from blockHeadersProvider', async function test() {
      blockHeadersSyncWorker.syncCheckpoint = chainHeight;
      await blockHeadersSyncWorker.execute();

      const errorCallback = this.sinon.spy();
      blockHeadersSyncWorker.parentEvents.on('error', errorCallback);
      const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;
      const error = new Error('Test error');
      blockHeadersProvider.emit('error', error);

      expect(errorCallback).to.have.been.calledOnceWith(error);
    });
  });

  describe('#onStop', () => {
    beforeEach(function beforeEach() {
      blockHeadersSyncWorker = createBlockHeadersSyncWorker(this.sinon);
    });

    it('should stop historical sync', async () => {
      const promise = blockHeadersSyncWorker.onStart();
      await waitOneTick();

      const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;

      blockHeadersSyncWorker.syncCheckpoint = 4;

      await blockHeadersSyncWorker.onStop();
      await promise;

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

      expect(blockHeadersSyncWorker.syncState).to.equal(BlockHeadersSyncWorker.STATES.IDLE);
      expect(blockHeadersSyncWorker.syncCheckpoint).to.equal(4);
    });

    it('should stop continuous sync', async () => {
      blockHeadersSyncWorker.syncCheckpoint = 1000;
      await blockHeadersSyncWorker.execute();
      await blockHeadersSyncWorker.onStop();

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

      expect(blockHeadersSyncWorker.syncState).to.equal(BlockHeadersSyncWorker.STATES.IDLE);
    });

    it('should continue historical sync from checkpoint after the restart', async () => {
      let startPromise = blockHeadersSyncWorker.onStart();

      const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;

      await waitOneTick();
      await blockHeadersSyncWorker.onStop();
      await startPromise;

      blockHeadersSyncWorker.syncCheckpoint = 980;

      startPromise = blockHeadersSyncWorker.onStart();

      expect(blockHeadersProvider.readHistorical).to.have.been.calledTwice;
      expect(blockHeadersProvider.readHistorical.secondCall)
        .to.have.been.calledWith(980, 1000);

      blockHeadersProvider.emit(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);
      await startPromise;
    });

    it('should continue continuous sync from checkpoint after the restart', async () => {
      const { blockHeadersProvider } = blockHeadersSyncWorker.transport.client;

      blockHeadersSyncWorker.syncCheckpoint = 1000;
      await blockHeadersSyncWorker.execute();
      blockHeadersSyncWorker.syncCheckpoint = 1200;
      await blockHeadersSyncWorker.onStop();

      blockHeadersSyncWorker.storage.getDefaultChainStore().state.chainHeight = 1200;

      await blockHeadersSyncWorker.execute();
      expect(blockHeadersProvider.startContinuousSync.secondCall)
        .to.have.been.calledWith(1200);
    });
  });

  describe('#continuousChainUpdateHandler', () => {
    beforeEach(function beforeEach() {
      blockHeadersSyncWorker = createBlockHeadersSyncWorker(this.sinon);
      blockHeadersSyncWorker.storage.getDefaultChainStore().state.lastSyncedHeaderHeight = 1000;
    });

    it('should store new headers and update the chain height', async () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      const headers = mockHeadersChain('testnet', 3, spvChainHeaders[spvChainHeaders.length - 1]);

      await blockHeadersSyncWorker.continuousChainUpdateHandler(headers, 1001);

      expect(chainStore.updateChainHeight).to.have.been.calledWith(1003);
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.been.calledWith(1003);
      expect(chainStore.updateHeadersMetadata).to.have.been.calledWith(headers, 1003);
      expect(chainStore.setBlockHeaders).to.have.been.calledWith(headers);
      expect(chainStore.rewindHeaders).to.have.not.been.called;
      expect(blockHeadersSyncWorker.parentEvents.emit)
        .to.have.been.calledWith(EVENTS.BLOCKHEIGHT_CHANGED, 1003);
      expect(blockHeadersSyncWorker.storage.scheduleStateSave)
        .to.have.been.called;
    });

    it('should replace the stored tip in a reorg at the same height', async () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      const headers = mockHeadersChain('testnet', 1);

      await blockHeadersSyncWorker.continuousChainUpdateHandler(headers, 1000);

      expect(chainStore.rewindHeaders).to.have.been.calledWith(1000);
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.been.calledWith(1000);
      expect(chainStore.state.chainHeight).to.equal(1000);
      expect(blockHeadersSyncWorker.parentEvents.emit)
        .to.have.been.calledWith(EVENTS.BLOCKHEIGHT_CHANGED, 1000);
    });

    it('should lower the chain height after a reorg to a chain with fewer headers', async () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();

      await blockHeadersSyncWorker.continuousChainUpdateHandler(mockHeadersChain('testnet', 1), 900);

      expect(chainStore.rewindHeaders).to.have.been.calledWith(900);
      expect(chainStore.state.chainHeight).to.equal(900);
      expect(chainStore.updateChainHeight).to.have.not.been.called;
      expect(blockHeadersSyncWorker.parentEvents.emit)
        .to.have.been.calledWith(EVENTS.BLOCKHEIGHT_CHANGED, 900);
    });

    it('should not lower the chain height without a reorg', async () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      chainStore.state.chainHeight = 1010;

      await blockHeadersSyncWorker.continuousChainUpdateHandler(mockHeadersChain('testnet', 1), 1001);

      expect(chainStore.state.chainHeight).to.equal(1010);
      expect(chainStore.updateChainHeight).to.have.not.been.called;
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.been.calledWith(1001);
    });

    it('should emit error in case headers array is empty', async () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();

      blockHeadersSyncWorker.parentEvents.on('error', () => {});
      await blockHeadersSyncWorker.continuousChainUpdateHandler([], 1010);

      const { args } = blockHeadersSyncWorker.parentEvents.emit.firstCall;
      expect(args[0]).to.equal('error');
      expect(args[1].message).to.equal('Invalid chain update of 0 headers at height 1010');

      expect(chainStore.updateChainHeight).to.have.not.been.called;
      expect(chainStore.setBlockHeaders).to.have.not.been.called;
      expect(blockHeadersSyncWorker.storage.scheduleStateSave)
        .to.have.not.been.called;
    });

    it('should emit error in case something goes wrong', async function test() {
      const error = new Error('Chain store was not found');
      blockHeadersSyncWorker.storage.getDefaultChainStore = () => {
        throw error;
      };

      const errorHandler = this.sinon.spy();
      blockHeadersSyncWorker.parentEvents.on('error', errorHandler);
      await blockHeadersSyncWorker.continuousChainUpdateHandler(
        mockHeadersChain('testnet', 1),
        1020,
      );
      expect(errorHandler).to.have.been.calledWith(error);
    });
  });

  describe('#historicalChainUpdateHandler', () => {
    beforeEach(function beforeEach() {
      blockHeadersSyncWorker = createBlockHeadersSyncWorker(this.sinon);
    });

    it('should store the headers the chain connected after the stored ones', () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      const stored = spvChainHeaders.slice(0, 4);
      chainStore.state.blockHeaders = stored;
      chainStore.state.lastSyncedHeaderHeight = 3;
      const headers = [spvChainHeaders[4]];

      blockHeadersSyncWorker.historicalChainUpdateHandler(headers, 4);

      expect(chainStore.setBlockHeaders)
        .to.have.been.calledWith(spvChainHeaders.slice(2, 5));
      expect(chainStore.updateHeadersMetadata).to.have.been.calledWith(headers, 4);
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.been.calledWith(4);
      expect(blockHeadersSyncWorker.syncCheckpoint).to.equal(4);
      expect(blockHeadersSyncWorker.storage.scheduleStateSave).to.have.been.called;
      expect(blockHeadersSyncWorker.scheduleProgressUpdate).to.have.been.called;
    });

    it('should fill a gap before the new headers from the spv chain', () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      chainStore.state.blockHeaders = spvChainHeaders.slice(0, 2);
      chainStore.state.lastSyncedHeaderHeight = 1;
      const headers = [spvChainHeaders[4]];

      blockHeadersSyncWorker.historicalChainUpdateHandler(headers, 4);

      expect(chainStore.setBlockHeaders).to.have.been.calledWith(spvChainHeaders.slice(-3));
      expect(chainStore.updateHeadersMetadata)
        .to.have.been.calledWith(spvChainHeaders.slice(2), 4);
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.been.calledWith(4);
    });

    it('should start the stored headers over when the gap can no longer be filled', () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      chainStore.state.blockHeaders = spvChainHeaders.slice(0, 2);
      chainStore.state.lastSyncedHeaderHeight = 1;
      const headers = mockHeadersChain('testnet', 2, spvChainHeaders[4]).slice(1);

      blockHeadersSyncWorker.historicalChainUpdateHandler(headers, 10);

      expect(chainStore.setBlockHeaders).to.have.been.calledWith(headers);
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.been.calledWith(10);
    });

    it('should emit error for an update without headers', function test() {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      const errorCallback = this.sinon.spy();
      blockHeadersSyncWorker.parentEvents.on('error', errorCallback);

      blockHeadersSyncWorker.historicalChainUpdateHandler([], 5);

      expect(errorCallback.firstCall.args[0].message)
        .to.equal('Invalid chain update of 0 headers at height 5');
      expect(chainStore.setBlockHeaders).to.have.not.been.called;
      expect(blockHeadersSyncWorker.storage.scheduleStateSave).to.have.not.been.called;
    });
  });

  describe('#updateProgress', () => {
    beforeEach(function beforeEach() {
      blockHeadersSyncWorker = createBlockHeadersSyncWorker(this.sinon);
    });

    it('should emit progress event when chain started from genesis', () => {
      blockHeadersSyncWorker.updateProgress();

      const { firstCall } = blockHeadersSyncWorker.parentEvents.emit;
      expect(firstCall).to.have.been.calledWith(EVENTS.HEADERS_SYNC_PROGRESS, {
        confirmedProgress: 0.4,
        totalProgress: 0.4,
        confirmedSyncedCount: 4,
        totalSyncedCount: 4,
        totalCount: 1000,
      });
    });

    // TODO(spv): cover cases with the orphan chunks?
  });
});
