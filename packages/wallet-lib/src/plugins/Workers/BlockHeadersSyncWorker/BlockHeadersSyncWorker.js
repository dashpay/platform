const BlockHeadersProvider = require('@dashevo/dapi-client/lib/BlockHeadersProvider/BlockHeadersProvider');
const Worker = require('../../Worker');
const logger = require('../../../logger');
const EVENTS = require('../../../EVENTS');

const PROGRESS_UPDATE_INTERVAL = 1000;

const MIN_HEADERS_TO_KEEP = 2;
const MAX_HEADERS_TO_KEEP = 5000;

const STATES = {
  IDLE: 'STATE_IDLE',
  HISTORICAL_SYNC: 'STATE_HISTORICAL_SYNC',
  CONTINUOUS_SYNC: 'STATE_CONTINUOUS_SYNC',
};

class BlockHeadersSyncWorker extends Worker {
  constructor(options = {}) {
    super({
      name: 'BlockHeadersSyncWorker',
      executeOnStart: true,
      firstExecutionRequired: true,
      awaitOnInjection: true,
      workerIntervalTime: 0,
      dependencies: [
        'network',
        'transport',
        'storage',
        'walletId',
      ],
      ...options,
    });

    if (options.maxHeadersToKeep && typeof options.maxHeadersToKeep !== 'number') {
      throw new Error(`Invalid maxHeadersToKeep "${options.maxHeadersToKeep}"`);
    }
    this.maxHeadersToKeep = options.maxHeadersToKeep || MAX_HEADERS_TO_KEEP;

    if (this.maxHeadersToKeep < MIN_HEADERS_TO_KEEP) {
      throw new Error(`Max headers to keep must be greater than ${MIN_HEADERS_TO_KEEP}, got ${this.maxHeadersToKeep}`);
    }

    this.syncCheckpoint = -1;
    this.progressUpdateTimeout = null;
    this.syncState = STATES.IDLE;

    this.blockHeadersProviderErrorHandler = null;
    this.historicalDataObtainedHandler = null;
    this.blockHeadersProviderStopHandler = null;

    this.updateProgress = this.updateProgress.bind(this);
    this.historicalChainUpdateHandler = this.historicalChainUpdateHandler.bind(this);
    this.continuousChainUpdateHandler = this.continuousChainUpdateHandler.bind(this);
  }

  inject(name, obj, allowSensitiveOperations = false) {
    super.inject(name, obj, allowSensitiveOperations);

    if (name === 'walletId') {
      this.logger = logger.getForWallet(this.walletId);
    }
  }

  async onStart() {
    if (this.syncState !== STATES.IDLE) {
      throw new Error(`Worker is already running: ${this.syncState}. Please call .onStop() first.`);
    }

    const chainStore = this.storage.getDefaultChainStore();
    let startFrom = this.getStartBlockHeight();
    if (startFrom < this.syncCheckpoint) {
      startFrom = this.syncCheckpoint;
    }

    const bestBlockHeight = typeof chainStore.state.chainHeight === 'number'
      ? chainStore.state.chainHeight : -1;

    if (bestBlockHeight < 1) {
      throw new Error(`Invalid best block height ${bestBlockHeight}`);
    } else if (startFrom > bestBlockHeight) {
      throw new Error(`Start block height ${startFrom} is greater than best block height ${bestBlockHeight}`);
    }

    const { blockHeadersProvider } = this.transport.client;
    blockHeadersProvider.on(
      BlockHeadersProvider.EVENTS.CHAIN_UPDATED,
      this.historicalChainUpdateHandler,
    );

    let stopped = false;
    const historicalSyncPromise = new Promise((resolve, reject) => {
      this.blockHeadersProviderErrorHandler = (e) => reject(e);

      this.blockHeadersProviderStopHandler = () => {
        blockHeadersProvider.removeListener(
          BlockHeadersProvider.EVENTS.CHAIN_UPDATED,
          this.historicalChainUpdateHandler,
        );
        blockHeadersProvider.removeListener(
          BlockHeadersProvider.EVENTS.ERROR,
          this.blockHeadersProviderErrorHandler,
        );
        blockHeadersProvider.removeListener(
          BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED,
          this.historicalDataObtainedHandler,
        );

        stopped = true;
        resolve();
      };

      this.historicalDataObtainedHandler = () => {
        blockHeadersProvider.removeListener(
          BlockHeadersProvider.EVENTS.CHAIN_UPDATED,
          this.historicalChainUpdateHandler,
        );
        blockHeadersProvider.removeListener(
          BlockHeadersProvider.EVENTS.ERROR,
          this.blockHeadersProviderErrorHandler,
        );
        blockHeadersProvider.removeListener(
          BlockHeadersProvider.EVENTS.STOPPED,
          this.blockHeadersProviderStopHandler,
        );

        resolve();
      };

      blockHeadersProvider.on(
        BlockHeadersProvider.EVENTS.ERROR,
        this.blockHeadersProviderErrorHandler,
      );
      blockHeadersProvider.once(
        BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED,
        this.historicalDataObtainedHandler,
      );
      blockHeadersProvider.once(
        BlockHeadersProvider.EVENTS.STOPPED,
        this.blockHeadersProviderStopHandler,
      );
    });

    this.logger.debug(`[BlockHeadersSyncWorker] Start reading historical headers from ${startFrom} to ${bestBlockHeight}`);
    await blockHeadersProvider.readHistorical(startFrom, bestBlockHeight);
    this.syncState = STATES.HISTORICAL_SYNC;

    await historicalSyncPromise;

    this.updateProgress();

    await this.storage.saveState();

    if (!stopped) {
      this.syncCheckpoint = bestBlockHeight;
    }
    this.syncState = STATES.IDLE;
  }

  async execute() {
    if (this.syncState !== STATES.IDLE) {
      throw new Error(`Worker is already running: ${this.syncState}. Please call .onStop() first.`);
    }

    const { blockHeadersProvider } = this.transport.client;
    blockHeadersProvider.on(
      BlockHeadersProvider.EVENTS.CHAIN_UPDATED,
      this.continuousChainUpdateHandler,
    );

    this.blockHeadersProviderErrorHandler = (e) => {
      this.emitError(e);
      this.logger.debug('[BlockHeadersSyncWorker] Error handling continuous chain update', e);
    };

    blockHeadersProvider.on(
      BlockHeadersProvider.EVENTS.ERROR,
      this.blockHeadersProviderErrorHandler,
    );

    this.blockHeadersProviderStopHandler = () => {
      blockHeadersProvider.removeListener(
        BlockHeadersProvider.EVENTS.CHAIN_UPDATED,
        this.continuousChainUpdateHandler,
      );

      blockHeadersProvider.removeListener(
        BlockHeadersProvider.EVENTS.ERROR,
        this.blockHeadersProviderErrorHandler,
      );

      this.syncState = STATES.IDLE;
    };

    blockHeadersProvider.once(
      BlockHeadersProvider.EVENTS.STOPPED,
      this.blockHeadersProviderStopHandler,
    );

    await blockHeadersProvider.startContinuousSync(this.syncCheckpoint);
    this.syncState = STATES.CONTINUOUS_SYNC;
  }

  async onStop() {
    this.logger.debug('[BlockHeadersSyncWorker] Stopping...');
    const { blockHeadersProvider } = this.transport.client;
    await blockHeadersProvider.stop();
  }

  /**
   * Determines starting point considering options
   * and last save checkpoint
   * @returns {number|number}
   */
  getStartBlockHeight() {
    const chainStore = this.storage.getDefaultChainStore();
    const bestBlockHeight = chainStore.state.chainHeight;

    let height;

    const {
      skipSynchronizationBeforeHeight,
      skipSynchronization,
    } = (this.storage.application.syncOptions || {});

    if (skipSynchronization) {
      this.logger.debug(`[BlockHeadersSyncWorker] Wallet created from a new mnemonic. Sync only last ${this.maxHeadersToKeep} blocks.`);
      const syncFrom = bestBlockHeight - this.maxHeadersToKeep;
      return syncFrom < 1 ? 1 : syncFrom;
    }

    const { lastSyncedHeaderHeight } = chainStore.state;

    if (typeof lastSyncedHeaderHeight !== 'number') {
      throw new Error(`Invalid last synced header height ${lastSyncedHeaderHeight}`);
    }

    const skipBefore = parseInt(skipSynchronizationBeforeHeight, 10);

    if (skipBefore > lastSyncedHeaderHeight) {
      this.logger.debug(`[BlockHeadersSyncWorker] UNSAFE option skipSynchronizationBeforeHeight is set to ${skipBefore}`);
      height = skipBefore;
    } else if (lastSyncedHeaderHeight > -1) {
      this.logger.debug(`[BlockHeadersSyncWorker] Last synced header height is ${lastSyncedHeaderHeight}`);
      height = lastSyncedHeaderHeight;
    } else {
      height = 1;
    }

    return height;
  }

  /**
   * Listens for chain updates during the synchronization of historical headers
   */
  historicalChainUpdateHandler() {
    try {
      const chainStore = this.storage.getDefaultChainStore();
      const { blockHeadersProvider } = this.transport.client;
      const { spvChain } = blockHeadersProvider;

      // TODO(spv): return only new headers added instead of the whole chain
      const longestChain = spvChain.getLongestChain({ withPruned: true });
      const { startBlockHeight } = spvChain;
      const { lastSyncedHeaderHeight } = chainStore.state;

      // TODO(spv): abstract this in spv chain?
      const totalHeadersCount = startBlockHeight + longestChain.length - 1; // Ignore genesis block
      const syncedHeadersCount = lastSyncedHeaderHeight;

      if (syncedHeadersCount > totalHeadersCount) {
        throw new Error(`Synced headers count ${syncedHeadersCount} is greater than total headers count ${totalHeadersCount}.`);
      }

      if (syncedHeadersCount < totalHeadersCount) {
        // Update headers in the store
        chainStore.setBlockHeaders(longestChain.slice(-this.maxHeadersToKeep));

        const newLastSyncedHeaderHeight = totalHeadersCount;
        const newHeaders = longestChain.slice(-(totalHeadersCount - syncedHeadersCount + 1));

        chainStore.updateHeadersMetadata(newHeaders, newLastSyncedHeaderHeight);
        chainStore.updateLastSyncedHeaderHeight(newLastSyncedHeaderHeight);
        this.syncCheckpoint = newLastSyncedHeaderHeight;

        this.storage.scheduleStateSave();
      }

      this.scheduleProgressUpdate();
    } catch (e) {
      this.emitError(e);
      this.logger.debug('[BlockHeadersSyncWorker] Error handling historical chain update:', e);
    }
  }

  async continuousChainUpdateHandler(newHeaders, batchHeadHeight) {
    try {
      const chainStore = this.storage.getDefaultChainStore();

      if (typeof batchHeadHeight !== 'number' || Number.isNaN(batchHeadHeight)) {
        throw new Error(`Invalid batch head height ${batchHeadHeight}`);
      }

      if (!newHeaders || !newHeaders.length) {
        throw new Error(`No new headers received for batch at height ${batchHeadHeight}`);
      }

      const newChainHeight = batchHeadHeight + newHeaders.length - 1;

      const { chainHeight } = chainStore.state;
      // Ignore height overlap in case of the stream reconnected
      if (newChainHeight === chainHeight) {
        this.logger.debug(`[BlockHeadersSyncWorker] New chain height ${newChainHeight} is equal to current one: ${chainHeight}`);
        return;
      } if (newChainHeight < chainHeight) {
        throw new Error(`New chain height ${newChainHeight} is less than latest height ${chainHeight}`);
      }

      const { blockHeadersProvider: { spvChain } } = this.transport.client;
      // TODO(spv): request only new headers instead of the whole chain
      const longestChain = spvChain.getLongestChain({ withPruned: true });

      chainStore.updateChainHeight(newChainHeight);
      chainStore.updateLastSyncedHeaderHeight(newChainHeight);
      chainStore.setBlockHeaders(longestChain.slice(-this.maxHeadersToKeep));
      chainStore.updateHeadersMetadata(newHeaders, newChainHeight);
      const header = newHeaders[newHeaders.length - 1];

      this.logger.debug(`[BlockHeadersSyncWorker] Chain height updated: ${newChainHeight}`);
      this.logger.debug(`[--------------------->] Validity chain length: ${spvChain.getLongestChain().length}`);
      this.logger.debug(`[--------------------->] New block hash: ${header.hash}`);

      this.parentEvents.emit(EVENTS.BLOCKHEIGHT_CHANGED, newChainHeight);

      this.storage.scheduleStateSave();
    } catch (e) {
      this.emitError(e);
      this.logger.debug('[BlockHeadersSyncWorker] Error handling continuous chain update', e);
    }
  }

  updateProgress() {
    if (this.progressUpdateTimeout) {
      clearTimeout(this.progressUpdateTimeout);
      this.progressUpdateTimeout = null;
    }

    const chainStore = this.storage.getDefaultChainStore();
    const { blockHeadersProvider } = this.transport.client;
    // TODO(spv): consider caching progress data in historicalChainUpdateHandler
    // and use values from there instead of duplicated computation
    const longestChain = blockHeadersProvider.spvChain.getLongestChain({ withPruned: true });
    const { orphanChunks, startBlockHeight } = blockHeadersProvider.spvChain;
    const totalOrphans = orphanChunks.reduce((sum, chunk) => sum + chunk.length, 0);

    const totalCount = chainStore.state.chainHeight; // Including root block

    // TODO(spv): hide these calculations in the SPVChain
    const confirmedSyncedCount = startBlockHeight + longestChain.length - 1; // Ignore genesis block
    const totalSyncedCount = confirmedSyncedCount + totalOrphans;

    const confirmedProgress = Math.round((confirmedSyncedCount / totalCount) * 1000) / 10;
    const totalProgress = Math.round((totalSyncedCount / totalCount) * 1000) / 10;

    this.logger.debug('[BlockHeadersSyncWorker] Historical fetch progress.');
    this.logger.debug(`[--------------------->] Confirmed: ${confirmedSyncedCount}/${totalCount}, ${confirmedProgress}%`);
    this.logger.debug(`[--------------------->] Total: ${totalSyncedCount}/${totalCount}, ${totalProgress}%`);
    if (confirmedProgress === 100) {
      this.logger.debug(`[--------------------->] Last header: ${longestChain[longestChain.length - 1].hash}`);
    }

    this.parentEvents.emit(EVENTS.HEADERS_SYNC_PROGRESS, {
      confirmedProgress,
      totalProgress,
      confirmedSyncedCount,
      totalSyncedCount,
      totalCount,
    });
  }

  scheduleProgressUpdate() {
    if (!this.progressUpdateTimeout) {
      this.progressUpdateTimeout = setTimeout(this.updateProgress, PROGRESS_UPDATE_INTERVAL);
    }
  }

  emitError(e) {
    this.parentEvents.emit('error', e);
  }
}

BlockHeadersSyncWorker.MAX_HEADERS_TO_KEEP = MAX_HEADERS_TO_KEEP;
BlockHeadersSyncWorker.STATES = STATES;

module.exports = BlockHeadersSyncWorker;
