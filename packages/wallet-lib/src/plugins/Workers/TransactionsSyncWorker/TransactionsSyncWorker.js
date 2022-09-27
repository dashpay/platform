const Worker = require('../../Worker');
const logger = require('../../../logger');

const STATES = {
  IDLE: 'STATE_IDLE',
  HISTORICAL_SYNC: 'STATE_HISTORICAL_SYNC',
  CONTINUOUS_SYNC: 'STATE_CONTINUOUS_SYNC',
};

class TransactionsSyncWorker extends Worker {
  constructor(options = {}) {
    super({
      name: 'TransactionsSyncWorker',
      executeOnStart: true,
      firstExecutionRequired: true,
      awaitOnInjection: true,
      workerIntervalTime: 0,
      dependencies: [
        // 'importTransactions',
        // 'importInstantLock',
        // 'storage',
        // 'keyChainStore',
        // 'transport',
        // 'walletId',
        // 'getAddress',
        // 'network',
        // 'index',
        // 'BIP44PATH',
        // 'walletType',
      ],
      ...options,
    });

    this.historicalSyncStream = null;
    this.continuousSyncStream = null;
    this.syncCheckpoint = -1;

    this.syncState = STATES.IDLE;
  }

  async onStart() {

  }

  async execute() {

  }

  async onStop() {

  }

  /**
   * Determines starting point considering options
   * and last save checkpoint
   * @returns {number|number}
   */
  getStartBlockHeight() {
    const chainStore = this.storage.getDefaultChainStore();
    // const bestBlockHeight = chainStore.state.chainHeight;
    let height;

    const {
      skipSynchronizationBeforeHeight,
    } = (this.storage.application.syncOptions || {});

    const { lastSyncedBlockHeight } = chainStore.state;

    if (typeof lastSyncedBlockHeight !== 'number') {
      throw new Error(`Invalid last synced header height ${lastSyncedBlockHeight}`);
    }

    const skipBefore = parseInt(skipSynchronizationBeforeHeight, 10);

    if (skipBefore > lastSyncedBlockHeight) {
      logger.debug(`[TransactionsSyncWorker] UNSAFE option skipSynchronizationBeforeHeight is set to ${skipBefore}`);
      height = skipBefore;
    } else if (lastSyncedBlockHeight > -1) {
      logger.debug(`[TransactionsSyncWorker] Last synced block height is ${lastSyncedBlockHeight}`);
      height = lastSyncedBlockHeight;
    } else {
      height = 1;
    }

    return height;
  }
}

TransactionsSyncWorker.STATES = STATES;

module.exports = TransactionsSyncWorker;
