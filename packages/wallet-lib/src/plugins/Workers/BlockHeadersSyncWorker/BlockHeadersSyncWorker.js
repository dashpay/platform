const BlockHeadersProvider = require('@dashevo/dapi-client/lib/BlockHeadersProvider/BlockHeadersProvider');
const { Block } = require('@dashevo/dashcore-lib');
const Worker = require('../../Worker');
const logger = require('../../../logger');
const EVENTS = require('../../../EVENTS');

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

    // TODO: write tests
    const chainUpdateHandler = async (newHeaders, batchHeadHeight) => {
      try {
        const chainStore = this.storage.getChainStore(this.network.toString());
        const walletStore = this.storage.getWalletStore(this.walletId);

        let newChainHeight = batchHeadHeight;
        if (newHeaders.length > 1) {
          newChainHeight += newHeaders.length - 1;
        }

        const { blockHeight } = chainStore.state;
        // Ignore height overlap in case of the stream reconnected
        if (newChainHeight === blockHeight) {
          return;
        } if (newChainHeight < blockHeight) {
          this.parentEvents.emit(new Error(`New chain height ${newChainHeight} is less than latest height ${blockHeight}`));
          return;
        }

        const rawBlock = await this.transport.getBlockByHeight(newChainHeight);
        const block = new Block(rawBlock);

        // TODO: do we really need it having in mind that wallet holds lastKnownBlock?
        chainStore.state.blockHeight = newChainHeight;
        walletStore.updateLastKnownBlock(newChainHeight);
        this.parentEvents.emit(EVENTS.BLOCKHEIGHT_CHANGED, newChainHeight);
        this.parentEvents.emit(EVENTS.BLOCK, block, newChainHeight);
        logger.debug(`BlockHeadersSyncWorker - setting chain height ${newChainHeight}`);

        const { blockHeadersProvider: { spvChain } } = this.transport.client;
        const { prunedHeaders, orphanChunks } = spvChain;
        const longestChain = spvChain.getLongestChain();
        const totalOrphans = orphanChunks.reduce((sum, chunk) => sum + chunk.length, 0);
        const totalChainLength = prunedHeaders.length + longestChain.length + totalOrphans;

        console.log(`[BlockHeadersSyncWorker] Chain height update: ${newChainHeight}, Headers added: ${newHeaders.length}, Total length: ${totalChainLength}`);
        console.log(`[--------------------->] Longest: ${longestChain.length}, Pruned: ${prunedHeaders.length}. Orphans: ${totalOrphans}`);
        // TODO: implement with pruning in mind
        // this.storage.scheduleStateSave();
      } catch (e) {
        console.log(e);
        this.parentEvents.emit('error', e);
      }
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

    const totalOrphans = orphanChunks.reduce((sum, chunk) => sum + chunk.length, 0);
    const synchronizedHistoricalHeaders = longestChain.length
      + prunedHeaders.length
      + totalOrphans;

    // TODO: test
    let progress = (this.syncCheckpoint + synchronizedHistoricalHeaders - 1)
      / totalHistoricalHeaders;
    progress = Math.round(progress * 1000) / 1000;

    const fetchedHeaders = this.syncCheckpoint + synchronizedHistoricalHeaders - 1;

    console.log(`[BlockHeadersSyncWorker] Historical fetch: ${fetchedHeaders}/${totalHistoricalHeaders}. Progress: ${progress}`);
    console.log(`[--------------------->] Longest: ${longestChain.length}, Pruned: ${prunedHeaders.length}. Orphans: ${totalOrphans}`);
    if (progress === 1) {
      console.log(`[--------------------->] last header: ${longestChain[longestChain.length - 1].hash}`);
    }
  }

  scheduleProgressUpdate() {
    if (!this.progressUpdateTimeout) {
      this.progressUpdateTimeout = setTimeout(this.updateProgress, PROGRESS_UPDATE_INTERVAL);
    }
  }
}

module.exports = BlockHeadersSyncWorker;
