const BlockHeadersProvider = require('@dashevo/dapi-client/lib/BlockHeadersProvider/BlockHeadersProvider');
const Worker = require('../../Worker');
const logger = require('../../../logger');
const EVENTS = require('../../../EVENTS');
const deriveBlockHeadersResumeContext = require('../../../types/ChainStore/deriveBlockHeadersResumeContext');

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
      // Continue from the chain's own tip, which a reorg may have moved
      this.syncCheckpoint = blockHeadersProvider.spvChain.getTipHeight();
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
   * Determines the starting point from persisted authenticated headers.
   * @returns {number}
   */
  getStartBlockHeight() {
    const chainStore = this.storage.getDefaultChainStore();
    const resumeContext = deriveBlockHeadersResumeContext(chainStore.state);

    const {
      skipSynchronization,
      skipSynchronizationBeforeHeight,
    } = (this.storage.application.syncOptions || {});

    // Headers are validated as a chain from an authenticated root, so a
    // mid-chain start cannot be verified and these options cannot be honoured
    // here. They still apply to transaction synchronization, so say so rather
    // than let a caller assume header sync was skipped too.
    if (skipSynchronization || skipSynchronizationBeforeHeight) {
      this.logger.warn('[BlockHeadersSyncWorker] Header synchronization starts from the last authenticated header and ignores '
        + `${skipSynchronization ? 'skipSynchronization' : 'skipSynchronizationBeforeHeight'}; transaction synchronization still honours it`);
    }

    if (resumeContext.startBlockHeight > 1) {
      this.logger.debug(`[BlockHeadersSyncWorker] Last resumable header height is ${resumeContext.startBlockHeight}`);
    }

    return resumeContext.startBlockHeight;
  }

  /**
   * Stores headers the SPV chain connected from `firstHeight` on. Headers at
   * or below the last stored one come from a reorg and replace it.
   * @param {BlockHeader[]} headers
   * @param {number} firstHeight
   * @return {number} height of the last header
   */
  storeConnectedHeaders(headers, firstHeight) {
    const chainStore = this.storage.getDefaultChainStore();

    if (!Number.isInteger(firstHeight) || !headers || !headers.length) {
      throw new Error(`Invalid chain update of ${headers && headers.length} headers at height ${firstHeight}`);
    }

    if (firstHeight <= chainStore.state.lastSyncedHeaderHeight) {
      chainStore.rewindHeaders(firstHeight);
    }

    // Stored headers must be consecutive, ending at lastSyncedHeaderHeight.
    // Fill a gap before the new headers from what the SPV chain still keeps;
    // if it can't be filled, start the stored headers over.
    const { lastSyncedHeaderHeight } = chainStore.state;
    const { spvChain } = this.transport.client.blockHeadersProvider;
    const gap = [];
    for (let height = firstHeight - 1; height > lastSyncedHeaderHeight; height -= 1) {
      const header = spvChain.getHeaderAt(height);
      if (!header) {
        break;
      }
      gap.push(header);
    }
    const newHeaders = gap.reverse().concat(headers);
    const storedHeaders = firstHeight - gap.length === lastSyncedHeaderHeight + 1
      ? chainStore.state.blockHeaders
      : [];

    const lastHeight = firstHeight + headers.length - 1;
    chainStore.setBlockHeaders(storedHeaders.concat(newHeaders).slice(-this.maxHeadersToKeep));
    chainStore.updateHeadersMetadata(newHeaders, lastHeight);
    chainStore.updateLastSyncedHeaderHeight(lastHeight);
    this.syncCheckpoint = lastHeight;

    return lastHeight;
  }

  /**
   * Listens for chain updates during the synchronization of historical headers
   * @param {BlockHeader[]} headers
   * @param {number} firstHeight
   */
  historicalChainUpdateHandler(headers, firstHeight) {
    try {
      this.storeConnectedHeaders(headers, firstHeight);
      this.storage.scheduleStateSave();
      this.scheduleProgressUpdate();
    } catch (e) {
      this.emitError(e);
      this.logger.debug('[BlockHeadersSyncWorker] Error handling historical chain update:', e);
    }
  }

  /**
   * @param {BlockHeader[]} headers
   * @param {number} firstHeight
   */
  async continuousChainUpdateHandler(headers, firstHeight) {
    try {
      const chainStore = this.storage.getDefaultChainStore();
      const isReorg = firstHeight <= chainStore.state.lastSyncedHeaderHeight;
      const newChainHeight = this.storeConnectedHeaders(headers, firstHeight);

      if (newChainHeight > chainStore.state.chainHeight) {
        chainStore.updateChainHeight(newChainHeight);
      } else if (isReorg) {
        // A reorg to a chain with more work but no more headers
        chainStore.state.chainHeight = newChainHeight;
      }

      this.logger.debug(`[BlockHeadersSyncWorker] Chain height updated: ${newChainHeight}`);
      this.logger.debug(`[--------------------->] New block hash: ${headers[headers.length - 1].hash}`);

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
    const { spvChain } = blockHeadersProvider;
    const totalOrphans = spvChain.orphanChunks.reduce((sum, chunk) => sum + chunk.length, 0);

    const totalCount = chainStore.state.chainHeight; // Including root block

    const confirmedSyncedCount = spvChain.getTipHeight(); // Ignore genesis block
    const totalSyncedCount = confirmedSyncedCount + totalOrphans;

    const confirmedProgress = Math.round((confirmedSyncedCount / totalCount) * 1000) / 10;
    const totalProgress = Math.round((totalSyncedCount / totalCount) * 1000) / 10;

    this.logger.debug('[BlockHeadersSyncWorker] Historical fetch progress.');
    this.logger.debug(`[--------------------->] Confirmed: ${confirmedSyncedCount}/${totalCount}, ${confirmedProgress}%`);
    this.logger.debug(`[--------------------->] Total: ${totalSyncedCount}/${totalCount}, ${totalProgress}%`);
    if (confirmedProgress === 100) {
      this.logger.debug(`[--------------------->] Last header: ${spvChain.getTipHash()}`);
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
