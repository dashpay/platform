const BlockHeadersProvider = require('@dashevo/dapi-client/lib/BlockHeadersProvider/BlockHeadersProvider');
const Worker = require('../../Worker');
const logger = require('../../../logger');

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
      ],
      ...options,
    });

    this.lastSyncedBlockHeight = 1;
  }

  async onStart() {
    const {
      skipSynchronizationBeforeHeight,
      skipSynchronization,
    } = (this.storage.application.syncOptions || {});

    const bestBlockHeight = this.storage.getChainStore(this.network.toString())
      .state.blockHeight;

    if (skipSynchronization) {
      logger.debug('BlockHeadersSyncWorker - Wallet created from a new mnemonic. Sync from the best block height.');
      this.lastSyncedBlockHeight = bestBlockHeight;
      return;
    }
    // 388902 - in SPV
    // 389144 - in task
    const skipBefore = typeof skipSynchronizationBeforeHeight === 'number'
      ? skipSynchronizationBeforeHeight
      : parseInt(skipSynchronizationBeforeHeight, 10);

    if (skipBefore && !Number.isNaN(skipBefore)) {
      this.lastSyncedBlockHeight = skipBefore;
    }

    const startTime = Date.now();

    const { blockHeadersProvider } = this.transport.client;
    const historicalSyncPromise = new Promise((resolve, reject) => {
      blockHeadersProvider.on('error', (e) => {
        logger.error('BlockHeadersProvider error:', e);
        reject(e);
      });

      let longestChainLength = 0;

      blockHeadersProvider.on(BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED, resolve);

      blockHeadersProvider
        .on(BlockHeadersProvider.EVENTS.CHAIN_UPDATED, (longestChain, totalOrphans) => {
          longestChainLength = longestChain.length;

          const timePassed = (Date.now() - startTime) / 1000;
          const velocity = Math.round((longestChainLength + totalOrphans) / timePassed);
          const eta = Math.round((735722 / velocity) / 60);
          const totalBlocks = bestBlockHeight - this.lastSyncedBlockHeight;
          const timeLeft = Math.round(
            ((totalBlocks - longestChainLength - totalOrphans) / velocity) / 60,
          );

          console.log('Longest chain length', longestChainLength, totalOrphans, `velocity: ${velocity} blocks/sec,`, `ETA: ${timeLeft} min`);
        });
    });

    // Some numbers
    // 1 stream - velocity: 710 blocks/sec, ETA: 17 min
    // 2 streams - velocity: 1054 blocks/sec, ETA: 12 min
    // 5 steams - velocity: 1165 blocks/sec, ETA: 11 min
    // 10 streams - velocity: 1193 blocks/sec, ETA: 10 min
    // 20 streams - velocity: 1130 blocks/sec ETA: 11
    // 40 streams - velocity: 1115 blocks/sec ETA: 11
    // 80 streams - velocity: 1135 blocks/sec, ETA: 11 min

    console.log('Start worker', this.lastSyncedBlockHeight, bestBlockHeight);
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
    console.log('Stop worker');
  }
}

module.exports = BlockHeadersSyncWorker;
