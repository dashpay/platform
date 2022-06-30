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

    this.lastSyncedBlockHeight = 1;
    this.progressUpdateTimeout = null;
  }

  async onStart() {
    const chainStore = this.storage.getChainStore(this.network.toString());
    const bestBlockHeight = chainStore.state.blockHeight;

    const {
      skipSynchronizationBeforeHeight,
      skipSynchronization,
    } = (this.storage.application.syncOptions || {});

    if (skipSynchronization) {
      this.lastSyncedBlockHeight = bestBlockHeight;
      logger.debug('BlockHeadersSyncWorker - Wallet created from a new mnemonic. Sync from the best block height.');
      return;
    }

    const { lastKnownBlock } = this.storage.getWalletStore(this.walletId).state;
    const skipBefore = typeof skipSynchronizationBeforeHeight === 'number'
      ? skipSynchronizationBeforeHeight
      : parseInt(skipSynchronizationBeforeHeight, 10);

    if (skipBefore > lastKnownBlock.height) {
      this.lastSyncedBlockHeight = skipBefore;
    } else if (lastKnownBlock.height !== -1) {
      this.lastSyncedBlockHeight = lastKnownBlock.height;
    }

    const { blockHeadersProvider } = this.transport.client;
    const historicalSyncPromise = new Promise((resolve, reject) => {
      blockHeadersProvider.on('error', (e) => {
        // TODO: test this error
        logger.error('BlockHeadersProvider error:', e);
        reject(e);
      });

      const chainUpdateHandler = () => {
        this.scheduleProgressUpdate();
      };

      blockHeadersProvider.once(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED, () => {
        blockHeadersProvider
          .removeListener(BlockHeadersProvider.EVENTS.CHAIN_UPDATED, chainUpdateHandler);
        resolve();
      });

      blockHeadersProvider.on(BlockHeadersProvider.EVENTS.CHAIN_UPDATED, chainUpdateHandler);
    });

    try {
      await blockHeadersProvider.start(this.lastSyncedBlockHeight, bestBlockHeight);
    } catch (e) {
      console.log(e);
    }

    await historicalSyncPromise;
  }

  async execute() {
    console.log('Do continuos execution');
  }

  async onStop() {
    // TODO: handle cancellation of the plugins chain
    // in case we are in the phase of plugins preparation
    const { blockHeadersProvider } = this.transport.client;
    await blockHeadersProvider.stop();
    console.log('Stop worker');
  }

  scheduleProgressUpdate() {
    if (!this.progressUpdateTimeout) {
      this.progressUpdateTimeout = setTimeout(() => {
        const chainStore = this.storage.getChainStore(this.network.toString());
        const totalHistoricalHeaders = chainStore.state.blockHeight + 1; // Including genesis block

        const { blockHeadersProvider } = this.transport.client;
        const longestChain = blockHeadersProvider.spvChain.getLongestChain();
        const { prunedHeaders, orphanChunks } = blockHeadersProvider.spvChain;

        const synchronizedHistoricalHeaders = longestChain.length
          + prunedHeaders.length
          + orphanChunks.reduce((sum, chunk) => sum + chunk.length, 0);

        let progress = (this.lastSyncedBlockHeight + synchronizedHistoricalHeaders)
          / (chainStore.state.blockHeight + 1);
        progress = Math.round(progress * 1000) / 1000;

        console.log(synchronizedHistoricalHeaders, totalHistoricalHeaders, progress);

        this.progressUpdateTimeout = null;
      }, PROGRESS_UPDATE_INTERVAL);
    }
  }
}

module.exports = BlockHeadersSyncWorker;
