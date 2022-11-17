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
      addHeaders(headers) {
        if (!this.longestChain) {
          this.longestChain = [...spvChainHeaders];
        }

        this.longestChain = [...this.longestChain, ...headers];
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
            },
            updateLastSyncedHeaderHeight: sinon.spy(),
            updateChainHeight: sinon.spy(),
            setBlockHeaders: sinon.spy(),
            updateHeadersMetadata: sinon.spy(),
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
      storage.getDefaultChainStore().state.chainHeight = 1000;

      storage.application.syncOptions = {
        skipSynchronization: true,
      };

      let startBlockHeight = blockHeadersSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1);

      storage.getDefaultChainStore().state.chainHeight = 3000;
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
      expect(blockHeadersSyncWorker.syncCheckpoint).to.equal(chainHeight);
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
    });

    it('should update chain height with a single header', async () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      const { blockHeadersProvider: { spvChain } } = blockHeadersSyncWorker.transport.client;

      const headers = mockHeadersChain('testnet', 1, spvChainHeaders[spvChainHeaders.length - 1]);
      const longestChain = spvChain.getLongestChain();
      longestChain.push(headers[0]);
      const batchHeadHeight = 1010;
      await blockHeadersSyncWorker.continuousChainUpdateHandler(
        headers,
        1010,
      );

      expect(chainStore.updateChainHeight)
        .to.have.been.calledWith(batchHeadHeight);
      expect(chainStore.updateLastSyncedHeaderHeight)
        .to.have.been.calledWith(batchHeadHeight);

      expect(chainStore.setBlockHeaders)
        .to.have.been.calledWith(longestChain.slice(-3));
      expect(blockHeadersSyncWorker.storage.scheduleStateSave)
        .to.have.been.called;
    });

    it('should update chain height with an array of headers', async () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();

      const batchHeadHeight = 1010;
      const headers = mockHeadersChain('testnet', 3, spvChainHeaders[spvChainHeaders.length - 1]);
      const {
        blockHeadersProvider: {
          spvChain,
        },
      } = blockHeadersSyncWorker.transport.client;
      spvChain.addHeaders(headers);

      await blockHeadersSyncWorker.continuousChainUpdateHandler(
        headers,
        1010,
      );

      const newHeight = batchHeadHeight + headers.length - 1;
      expect(chainStore.updateChainHeight)
        .to.have.been.calledWith(newHeight);
      expect(chainStore.updateLastSyncedHeaderHeight)
        .to.have.been.calledWith(newHeight);
      expect(chainStore.setBlockHeaders)
        .to.have.been.calledWith(spvChain.getLongestChain().slice(-3));
      expect(blockHeadersSyncWorker.storage.scheduleStateSave)
        .to.have.been.called;
    });

    it('should do nothing if height hasn\'t changed', async () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      await blockHeadersSyncWorker.continuousChainUpdateHandler(
        mockHeadersChain('testnet', 2),
        999,
      );

      expect(chainStore.updateChainHeight).to.have.not.been.called;
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.not.been.called;
      expect(chainStore.setBlockHeaders).to.have.not.been.called;
      expect(blockHeadersSyncWorker.storage.scheduleStateSave)
        .to.have.not.been.called;
    });

    it('should emit error in case headers array is empty', async () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();

      const batchHeadHeight = 1010;
      blockHeadersSyncWorker.parentEvents.on('error', () => {});
      await blockHeadersSyncWorker.continuousChainUpdateHandler(
        [],
        batchHeadHeight,
      );

      const { args } = blockHeadersSyncWorker.parentEvents.emit.firstCall;
      expect(args[0]).to.equal('error');
      expect(args[1].message)
        .to.equal(`No new headers received for batch at height ${batchHeadHeight}`);

      expect(chainStore.updateChainHeight).to.have.not.been.called;
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.not.been.called;
      expect(chainStore.setBlockHeaders).to.have.not.been.called;
      expect(blockHeadersSyncWorker.storage.scheduleStateSave)
        .to.have.not.been.called;
    });

    it('should emit error in case new height is less than current height', async () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();

      const batchHeadHeight = 900;
      blockHeadersSyncWorker.parentEvents.on('error', () => {});
      await blockHeadersSyncWorker.continuousChainUpdateHandler(
        mockHeadersChain('testnet', 1),
        batchHeadHeight,
      );

      const { args } = blockHeadersSyncWorker.parentEvents.emit.firstCall;
      expect(args[0]).to.equal('error');
      expect(args[1].message)
        .to.equal('New chain height 900 is less than latest height 1000');

      expect(chainStore.updateChainHeight).to.have.not.been.called;
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.not.been.called;
      expect(chainStore.setBlockHeaders).to.have.not.been.called;
      expect(blockHeadersSyncWorker.storage.scheduleStateSave)
        .to.have.not.been.called;
    });

    it('should emit BLOCKHEIGHT_CHANGED event', async () => {
      const batchHeadHeight = 1020;
      await blockHeadersSyncWorker.continuousChainUpdateHandler(
        mockHeadersChain('testnet', 1),
        batchHeadHeight,
      );

      expect(blockHeadersSyncWorker.parentEvents.emit)
        .to.have.been.calledWith(EVENTS.BLOCKHEIGHT_CHANGED, batchHeadHeight);
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

    it('should update block headers', () => {
      const headers = mockHeadersChain('testnet', 3, spvChainHeaders[spvChainHeaders.length - 1]);
      const { blockHeadersProvider: { spvChain } } = blockHeadersSyncWorker.transport.client;
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      chainStore.state.lastSyncedHeaderHeight = 3;

      spvChain.addHeaders(headers);
      blockHeadersSyncWorker.historicalChainUpdateHandler();

      const longestChain = spvChain.getLongestChain();
      const newHeight = longestChain.length - 1;

      expect(chainStore.setBlockHeaders)
        .to.have.been.calledWith(longestChain.slice(-headers.length));
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.been.calledWith(newHeight);

      const newHeaders = longestChain.slice(-(longestChain.length - headers.length));
      expect(chainStore.updateHeadersMetadata)
        .to.have.been.calledWith(newHeaders, newHeight);

      expect(blockHeadersSyncWorker.syncCheckpoint)
        .to.equal(newHeight);

      expect(blockHeadersSyncWorker.storage.scheduleStateSave)
        .to.have.been.called;
      expect(blockHeadersSyncWorker.scheduleProgressUpdate)
        .to.have.been.called;
    });

    it('should do nothing in case amount of total headers hasn\'t changed', () => {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      chainStore.state.lastSyncedHeaderHeight = 4;

      blockHeadersSyncWorker.historicalChainUpdateHandler();

      expect(chainStore.setBlockHeaders).to.have.not.been.called;
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.not.been.called;
      expect(chainStore.updateHeadersMetadata).to.have.not.been.called;
      expect(blockHeadersSyncWorker.storage.scheduleStateSave)
        .to.have.not.been.called;
      expect(blockHeadersSyncWorker.scheduleProgressUpdate)
        .to.have.been.called;
    });

    it('should emit error in case syncedHeadersCount is bigger than total headers count', function test() {
      const chainStore = blockHeadersSyncWorker.storage.getDefaultChainStore();
      chainStore.state.lastSyncedHeaderHeight = 5;

      const errorCallback = this.sinon.spy();
      blockHeadersSyncWorker.parentEvents.on('error', errorCallback);

      blockHeadersSyncWorker.historicalChainUpdateHandler();

      const { firstCall } = errorCallback;
      expect(firstCall.args[0].message)
        .to.equal('Synced headers count 5 is greater than total headers count 4.');

      expect(chainStore.setBlockHeaders).to.have.not.been.called;
      expect(chainStore.updateLastSyncedHeaderHeight).to.have.not.been.called;
      expect(chainStore.updateHeadersMetadata).to.have.not.been.called;
      expect(blockHeadersSyncWorker.storage.scheduleStateSave)
        .to.have.not.been.called;
      expect(blockHeadersSyncWorker.scheduleProgressUpdate)
        .to.have.not.been.called;
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
