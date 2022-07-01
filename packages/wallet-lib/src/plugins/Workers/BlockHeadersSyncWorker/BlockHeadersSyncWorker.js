const BlockHeadersProvider = require('@dashevo/dapi-client/lib/BlockHeadersProvider/BlockHeadersProvider');
const Worker = require('../../Worker');
const logger = require('../../../logger');

const PROGRESS_UPDATE_INTERVAL = 1000;

class BlockHeadersSyncWorker extends Worker {
  constructor(options) {
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

    this.syncCheckpoint = 1;
    this.progressUpdateTimeout = null;
    this.updateProgress = this.updateProgress.bind(this);
  }

  async onStart() {
    const chainStore = this.storage.getChainStore(this.network.toString());
    const bestBlockHeight = chainStore.state.blockHeight;

    const {
      skipSynchronizationBeforeHeight,
      skipSynchronization,
    } = (this.storage.application.syncOptions || {});

    if (skipSynchronization) {
      this.syncCheckpoint = bestBlockHeight;
      logger.debug('BlockHeadersSyncWorker - Wallet created from a new mnemonic. Sync from the best block height.');
      return;
    }

    const { lastKnownBlock } = this.storage.getWalletStore(this.walletId).state;
    const skipBefore = typeof skipSynchronizationBeforeHeight === 'number'
      ? skipSynchronizationBeforeHeight
      : parseInt(skipSynchronizationBeforeHeight, 10);

    if (skipBefore > lastKnownBlock.height) {
      this.syncCheckpoint = skipBefore;
    } else if (lastKnownBlock.height !== -1) {
      this.syncCheckpoint = lastKnownBlock.height;
    }

    const { blockHeadersProvider } = this.transport.client;
    const historicalSyncPromise = new Promise((resolve, reject) => {
      const errorHandler = (e) => reject(e);
      const chainUpdateHandler = () => {
        this.scheduleProgressUpdate();
      };

      blockHeadersProvider.on(BlockHeadersProvider.EVENTS.CHAIN_UPDATED, chainUpdateHandler);
      blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, errorHandler);

      blockHeadersProvider.once(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED, () => {
        blockHeadersProvider.removeListener(BlockHeadersProvider.EVENTS.ERROR, errorHandler);
        blockHeadersProvider
          .removeListener(BlockHeadersProvider.EVENTS.CHAIN_UPDATED, chainUpdateHandler);
        resolve();
      });
    });

    try {
      await blockHeadersProvider.readHistorical(this.syncCheckpoint, bestBlockHeight);
    } catch (e) {
      console.log(e);
    }

    await historicalSyncPromise;
    this.updateProgress();
    this.syncCheckpoint = bestBlockHeight;
  }

  async execute() {
    const errorHandler = (e) => {
      this.parentEvents.emit('error', e);
    };

    const chainUpdateHandler = (newHeaders, headHeight) => {
      let newChainHeight = headHeight;
      if (newHeaders.length > 1) {
        newChainHeight += newHeaders.length - 1;
      }
      console.log('Height updated', newChainHeight);
    };

    const { blockHeadersProvider } = this.transport.client;
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.CHAIN_UPDATED, chainUpdateHandler);
    blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, errorHandler);

    await blockHeadersProvider.startContinuousSync(this.syncCheckpoint);
  }

  async onStop() {
    // TODO: handle cancellation of the plugins chain
    // in case we are in the phase of plugins preparation
    const { blockHeadersProvider } = this.transport.client;
    await blockHeadersProvider.stop();
    console.log('Stop worker');
  }

  updateProgress() {
    if (this.progressUpdateTimeout) {
      clearTimeout(this.progressUpdateTimeout);
      this.progressUpdateTimeout = null;
    }

    const chainStore = this.storage.getChainStore(this.network.toString());
    const totalHistoricalHeaders = chainStore.state.blockHeight + 1; // Including root block

    const { blockHeadersProvider } = this.transport.client;
    const longestChain = blockHeadersProvider.spvChain.getLongestChain();
    const { prunedHeaders, orphanChunks } = blockHeadersProvider.spvChain;

    const synchronizedHistoricalHeaders = longestChain.length
      + prunedHeaders.length
      + orphanChunks.reduce((sum, chunk) => sum + chunk.length, 0);

    // TODO: test
    let progress = (this.syncCheckpoint + synchronizedHistoricalHeaders - 1)
      / totalHistoricalHeaders;
    progress = Math.round(progress * 1000) / 1000;

    console.log(this.syncCheckpoint + synchronizedHistoricalHeaders - 1,
      totalHistoricalHeaders, progress);
  }

  scheduleProgressUpdate() {
    if (!this.progressUpdateTimeout) {
      this.progressUpdateTimeout = setTimeout(this.updateProgress, PROGRESS_UPDATE_INTERVAL);
    }
  }
}

module.exports = BlockHeadersSyncWorker;
