const BlockHeadersProvider = require('@dashevo/dapi-client/lib/BlockHeadersProvider/BlockHeadersProvider');
const { Block } = require('@dashevo/dashcore-lib');
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
        'importBlockHeader',
        'chainSyncMediator',
        'walletId',
      ],
      ...options,
    });

    this.maxHeadersToKeep = typeof options.maxHeadersToKeep === 'number'
      ? options.maxHeadersToKeep
      : MAX_HEADERS_TO_KEEP;

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

  async onStart() {
    if (this.syncState !== STATES.IDLE) {
      throw new Error(`Worker is already running: ${this.syncState}. Please call .onStop() first.`);
    }

    const chainStore = this.storage.getDefaultChainStore();
    let startFrom = this.getStartBlockHeight();
    if (startFrom < this.syncCheckpoint) {
      startFrom = this.syncCheckpoint;
    }

    const bestBlockHeight = typeof chainStore.state.blockHeight === 'number'
      ? chainStore.state.blockHeight : -1;

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

    await blockHeadersProvider.readHistorical(startFrom, bestBlockHeight);
    this.syncState = STATES.HISTORICAL_SYNC;

    await historicalSyncPromise;

    this.updateProgress();

    // TODO: cover with unit test
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

    const chainStore = this.storage.getDefaultChainStore();

    const bestBlockHeight = typeof chainStore.state.blockHeight === 'number'
      ? chainStore.state.blockHeight : -1;

    if (this.syncCheckpoint !== bestBlockHeight) {
      throw new Error(`Sync checkpoint is not equal to best block height: ${this.syncCheckpoint} !== ${bestBlockHeight}. Please read historical data first.`);
    }

    const { blockHeadersProvider } = this.transport.client;
    blockHeadersProvider.on(
      BlockHeadersProvider.EVENTS.CHAIN_UPDATED,
      this.continuousChainUpdateHandler,
    );

    this.blockHeadersProviderErrorHandler = (e) => {
      this.emitError(e);
      logger.debug('[BlockHeadersSyncWorker] Error handling continuous chain update', e);
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
    // TODO: handle cancellation of the plugins chain
    // in case we are in the phase of plugins preparation
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
    const bestBlockHeight = chainStore.state.blockHeight;

    let height;

    const {
      skipSynchronizationBeforeHeight,
      skipSynchronization,
    } = (this.storage.application.syncOptions || {});

    if (skipSynchronization) {
      logger.debug(`[BlockHeadersSyncWorker] Wallet created from a new mnemonic. Sync only last ${this.maxHeadersToKeep} blocks.`);
      const syncFrom = bestBlockHeight - this.maxHeadersToKeep;
      return syncFrom < 1 ? 1 : syncFrom;
    }

    const lastSyncedHeaderHeight = typeof chainStore.state.lastSyncedHeaderHeight === 'number'
      ? chainStore.state.lastSyncedHeaderHeight : -1;

    const skipBefore = typeof skipSynchronizationBeforeHeight === 'number'
      ? skipSynchronizationBeforeHeight : -1;

    if (skipBefore > lastSyncedHeaderHeight) {
      logger.debug(`[BlockHeadersSyncWorker] UNSAFE option skipSynchronizationBeforeHeight is set to ${skipBefore}`);
      height = skipBefore;
    } else if (lastSyncedHeaderHeight > -1) {
      logger.debug(`[BlockHeadersSyncWorker] Last synced header height is ${lastSyncedHeaderHeight}`);
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

      const longestChain = spvChain.getLongestChain({ withPruned: true });
      const { startBlockHeight } = spvChain;
      const { lastSyncedHeaderHeight } = chainStore.state;

      // TODO: abstract this in spv chain?
      const totalHeadersCount = startBlockHeight + longestChain.length;
      const syncedHeadersCount = lastSyncedHeaderHeight + 1;

      if (syncedHeadersCount > totalHeadersCount) {
        const error = new Error(`Synced headers count ${syncedHeadersCount} is greater than total headers count ${totalHeadersCount}.`);
        this.emitError(error);
        logger.debug('[BlockHeadersSyncWorker] Error handling historical chain update:', error);
        return;
      }

      if (syncedHeadersCount < totalHeadersCount) {
        // Update headers in the store
        chainStore.setBlockHeaders(longestChain.slice(-this.maxHeadersToKeep));

        const newLastSyncedHeaderHeight = totalHeadersCount - 1;
        const newHeaders = longestChain.slice(-(totalHeadersCount - syncedHeadersCount));

        chainStore.updateHeadersMetadata(newHeaders, newLastSyncedHeaderHeight);
        chainStore.updateLastSyncedHeaderHeight(newLastSyncedHeaderHeight);
        this.syncCheckpoint = newLastSyncedHeaderHeight;

        this.storage.scheduleStateSave();
      }

      this.scheduleProgressUpdate();
    } catch (e) {
      this.emitError(e);
      logger.debug('[BlockHeadersSyncWorker] Error handling historical chain update:', e);
    }
  }

  async continuousChainUpdateHandler(newHeaders, batchHeadHeight) {
    try {
      const chainStore = this.storage.getDefaultChainStore();
      const walletStore = this.storage.getDefaultWalletStore();

      // TODO: add test
      if (typeof batchHeadHeight !== 'number' || Number.isNaN(batchHeadHeight)) {
        const error = new Error(`Invalid batch head height ${batchHeadHeight}`);
        this.emitError(error);
        logger.debug('[BlockHeadersSyncWorker] Error handling continuous chain update:', error);
        return;
      }

      if (!newHeaders || !newHeaders.length) {
        const error = new Error(`No new headers received for batch at height ${batchHeadHeight}`);
        this.emitError(error);
        logger.debug('[BlockHeadersSyncWorker] Error handling continuous chain update:', error);
        return;
      }

      const newChainHeight = batchHeadHeight + newHeaders.length - 1;

      const { blockHeight } = chainStore.state;
      // Ignore height overlap in case of the stream reconnected
      if (newChainHeight === blockHeight) {
        logger.debug(`[BlockHeadersSyncWorker] New chain height ${newChainHeight} is equal to current one: ${blockHeight}`);
        return;
      } if (newChainHeight < blockHeight) {
        const error = new Error(`New chain height ${newChainHeight} is less than latest height ${blockHeight}`);
        this.emitError(error);
        logger.debug('[BlockHeadersSyncWorker] Error handling continuous chain update:', error);
        return;
      }

      // TODO: cover case where there are more than one block has been mined,
      // and perform this logic for every of them
      const rawBlock = await this.transport.getBlockByHeight(newChainHeight);
      const block = new Block(rawBlock);

      const { blockHeadersProvider: { spvChain } } = this.transport.client;
      const longestChain = spvChain.getLongestChain({ withPruned: true });

      // TODO: do we really need it having in mind that wallet holds lastKnownBlock?
      chainStore.updateChainHeight(newChainHeight);
      chainStore.updateLastSyncedHeaderHeight(newChainHeight);
      chainStore.setBlockHeaders(longestChain.slice(-this.maxHeadersToKeep));
      chainStore.updateHeadersMetadata(newHeaders, newChainHeight);
      walletStore.updateLastKnownBlock(newChainHeight);

      this.parentEvents.emit(EVENTS.BLOCKHEIGHT_CHANGED, newChainHeight);
      this.parentEvents.emit(EVENTS.BLOCK, block, newChainHeight);

      const { orphanChunks } = spvChain;
      const totalOrphans = orphanChunks.reduce((sum, chunk) => sum + chunk.length, 0);
      const totalChainLength = longestChain.length + totalOrphans;
      logger.debug(`[BlockHeadersSyncWorker] Chain height update: ${newChainHeight}, Headers added: ${newHeaders.length}, Total length: ${totalChainLength}`);
      logger.debug(`[--------------------->] Longest: ${longestChain.length}, Orphans: ${totalOrphans}`);

      this.storage.scheduleStateSave();
    } catch (e) {
      this.emitError(e);
      logger.debug('[BlockHeadersSyncWorker] Error handling continuous chain update', e);
    }
  }

  updateProgress() {
    if (this.progressUpdateTimeout) {
      clearTimeout(this.progressUpdateTimeout);
      this.progressUpdateTimeout = null;
    }

    const chainStore = this.storage.getDefaultChainStore();
    const { blockHeadersProvider } = this.transport.client;
    const longestChain = blockHeadersProvider.spvChain.getLongestChain({ withPruned: true });
    const { orphanChunks, startBlockHeight } = blockHeadersProvider.spvChain;
    const totalOrphans = orphanChunks.reduce((sum, chunk) => sum + chunk.length, 0);

    const totalCount = chainStore.state.blockHeight + 1; // Including root block

    // TODO: provide this data from SPVChain
    const confirmedSyncedCount = startBlockHeight + longestChain.length;
    const totalSyncedCount = confirmedSyncedCount + totalOrphans;

    const confirmedProgress = Math.round((confirmedSyncedCount / totalCount) * 1000) / 10;
    const totalProgress = Math.round((totalSyncedCount / totalCount) * 1000) / 10;

    logger.debug('[BlockHeadersSyncWorker] Historical fetch progress.');
    logger.debug(`[--------------------->] Confirmed: ${confirmedSyncedCount}/${totalCount}, ${confirmedProgress}%`);
    logger.debug(`[--------------------->] Total: ${totalSyncedCount}/${totalCount}, ${totalProgress}%`);
    if (confirmedProgress === 100) {
      logger.debug(`[--------------------->] Last header: ${longestChain[longestChain.length - 1].hash}`);
    }

    // TODO: add confirmedSynced, totalSynced and total count to the progress event
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

  // TODO: write unit tests
  emitError(e) {
    if (this.parentEvents.listenerCount('error') === 0) {
      logger.debug('[BlockHeadersSyncWorker] Unhandled parentEvents \'error\' event:', e);
      return;
    }

    this.parentEvents.emit('error', e);
  }
}

BlockHeadersSyncWorker.MAX_HEADERS_TO_KEEP = MAX_HEADERS_TO_KEEP;
BlockHeadersSyncWorker.STATES = STATES;

module.exports = BlockHeadersSyncWorker;
