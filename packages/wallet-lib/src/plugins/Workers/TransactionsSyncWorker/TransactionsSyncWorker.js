const BlockHeadersProvider = require('@dashevo/dapi-client/lib/BlockHeadersProvider/BlockHeadersProvider');
const Worker = require('../../Worker');
const logger = require('../../../logger');
const TransactionsReader = require('./TransactionsReader');
const { getAddressesToSync } = require('./utils');

const STATES = {
  IDLE: 'STATE_IDLE',
  HISTORICAL_SYNC: 'STATE_HISTORICAL_SYNC',
  CONTINUOUS_SYNC: 'STATE_CONTINUOUS_SYNC',
};

const MAX_RETRIES = 10;

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
        'keyChainStore',
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

    this.transactionsReader = new TransactionsReader({
      maxRetries: MAX_RETRIES,
    });

    this.historicalSyncStream = null;
    this.continuousSyncStream = null;
    this.syncCheckpoint = -1;

    this.transactionsReaderErrorHandler = null;
    this.transactionsReaderStoppedHandler = null;
    this.historicalDataObtainedHandler = null;

    this.historicalTransactionsHandler = this.historicalTransactionsHandler.bind(this);
    this.historicalMerkleBlockHandler = this.historicalMerkleBlockHandler.bind(this);
    this.newTransactionsHandler = this.historicalTransactionsHandler.bind(this);
    this.newMerkleBlockHandler = this.historicalMerkleBlockHandler.bind(this);

    this.syncState = STATES.IDLE;
  }

  async onStart() {
    const chainStore = this.storage.getDefaultChainStore();

    const { chainHeight } = chainStore.state;

    if (typeof chainHeight !== 'number') {
      throw new Error(`Chain height is not a number: "${chainHeight}"`);
    }

    let startFrom = this.getStartBlockHeight();
    if (startFrom < this.syncCheckpoint) {
      startFrom = this.syncCheckpoint;
    }

    if (chainHeight < 1) {
      throw new Error(`Invalid current chain height ${chainHeight}`);
    } else if (startFrom > chainHeight) {
      throw new Error(`Start block height ${startFrom} is greater than chain height ${chainHeight}`);
    } else if (startFrom === chainHeight) {
      logger.debug(`Start block height is equal to chain height ${chainHeight}, no need to sync`);
      chainStore.clearHeadersMetadata();
      return;
    }

    const {
      skipSynchronization,
    } = (this.storage.application.syncOptions || {});

    if (skipSynchronization) {
      logger.debug(`[TransactionSyncStreamWorker] Wallet created from a new mnemonic. Sync from the current chain height ${chainHeight}`);
      chainStore.updateLastSyncedBlockHeight(chainHeight);
      this.syncCheckpoint = chainHeight;
      chainStore.clearHeadersMetadata();
    }

    this.transactionsReader.on(
      TransactionsReader.EVENTS.HISTORICAL_TRANSACTIONS,
      this.historicalTransactionsHandler,
    );

    this.transactionsReader.on(
      TransactionsReader.EVENTS.MERKLE_BLOCK,
      this.historicalMerkleBlockHandler,
    );

    const historicalSyncPromise = new Promise((resolve, reject) => {
      this.transactionsReaderErrorHandler = (e) => reject(e);

      this.transactionsReaderStoppedHandler = () => {
        this.transactionsReader.removeListener(
          TransactionsReader.EVENTS.HISTORICAL_TRANSACTIONS,
          this.historicalTransactionsHandler,
        );

        this.transactionsReader.removeListener(
          TransactionsReader.EVENTS.MERKLE_BLOCK,
          this.historicalMerkleBlockHandler,
        );

        this.transactionsReader.removeListener(
          TransactionsReader.EVENTS.ERROR,
          this.transactionsReaderErrorHandler,
        );

        this.transactionsReader.removeListener(
          TransactionsReader.EVENTS.HISTORICAL_DATA_OBTAINED,
          this.historicalDataObtainedHandler,
        );

        resolve({
          stopped: true,
        });
      };

      this.historicalDataObtainedHandler = () => {
        this.transactionsReader.removeListener(
          TransactionsReader.EVENTS.HISTORICAL_TRANSACTIONS,
          this.historicalTransactionsHandler,
        );
        this.transactionsReader.removeListener(
          TransactionsReader.EVENTS.MERKLE_BLOCK,
          this.historicalMerkleBlockHandler,
        );
        this.transactionsReader.removeListener(
          BlockHeadersProvider.EVENTS.ERROR,
          this.transactionsReaderErrorHandler,
        );
        this.transactionsReader.removeListener(
          BlockHeadersProvider.EVENTS.STOPPED,
          this.transactionsReaderStoppedHandler,
        );

        resolve({
          stopped: false,
        });
      };

      this.transactionsReader.on(
        TransactionsReader.EVENTS.ERROR,
        this.transactionsReaderErrorHandler,
      );
      this.transactionsReader.once(
        TransactionsReader.EVENTS.HISTORICAL_DATA_OBTAINED,
        this.historicalDataObtainedHandler,
      );
      this.transactionsReader.once(
        TransactionsReader.EVENTS.STOPPED,
        this.transactionsReaderStoppedHandler,
      );
    });

    const addresses = getAddressesToSync(this.keyChainStore);

    await this.transactionsReader
      .startHistoricalSync(startFrom, chainHeight, addresses);

    this.syncState = STATES.HISTORICAL_SYNC;

    const syncResult = await historicalSyncPromise;

    this.updateProgress();
    this.storage.saveState();

    if (!syncResult.stopped) {
      chainStore.clearHeadersMetadata();
      this.syncCheckpoint = chainHeight;
    }

    this.syncState = STATES.IDLE;
  }

  async execute() {
    if (this.syncState !== STATES.IDLE) {
      throw new Error(`Worker is already running: ${this.syncState}. Please call .onStop() first.`);
    }

    this.transactionsReader.on(
      TransactionsReader.EVENTS.NEW_TRANSACTIONS,
      this.newTransactionsHandler,
    );

    this.transactionsReader.on(
      TransactionsReader.EVENTS.MERKLE_BLOCK,
      this.newMerkleBlockHandler,
    );

    this.transactionsReaderErrorHandler = (e) => {
      this.emitError(e);
      logger.debug('[TransactionsSyncWorker] Error handling continuous chain update', e);
    };

    this.transactionsReader.on(
      TransactionsReader.EVENTS.ERROR,
      this.transactionsReaderErrorHandler,
    );

    this.transactionsReaderStoppedHandler = () => {
      this.transactionsReader.removeListener(
        TransactionsReader.EVENTS.NEW_TRANSACTIONS,
        this.newTransactionsHandler,
      );

      this.transactionsReader.removeListener(
        TransactionsReader.EVENTS.MERKLE_BLOCK,
        this.newMerkleBlockHandler,
      );

      this.transactionsReader.removeListener(
        TransactionsReader.EVENTS.ERROR,
        this.transactionsReaderErrorHandler,
      );

      this.syncState = STATES.IDLE;
    };

    this.transactionsReader.once(
      TransactionsReader.EVENTS.STOPPED,
      this.transactionsReaderStoppedHandler,
    );

    const addresses = getAddressesToSync(this.keyChainStore);
    await this.transactionsReader
      .startContinuousSync(this.syncCheckpoint, addresses);

    this.syncState = STATES.CONTINUOUS_SYNC;
  }

  async onStop() {
    if (this.syncState === STATES.HISTORICAL_SYNC) {
      await this.transactionsReader.stopHistoricalSync();
    } else if (this.syncState === STATES.CONTINUOUS_SYNC) {
      await this.transactionsReader.stopContinuousSync();
    }
  }

  /**
   * Determines starting point considering options
   * and last save checkpoint
   * @returns {number|number}
   */
  getStartBlockHeight() {
    const chainStore = this.storage.getDefaultChainStore();
    const { lastSyncedBlockHeight, chainHeight } = chainStore.state;

    let height;

    const {
      skipSynchronizationBeforeHeight,
      skipSynchronization,
    } = (this.storage.application.syncOptions || {});

    if (skipSynchronization) {
      logger.debug(`[TransactionsSyncWorker] Wallet created from a new mnemonic. Sync from current chain height ${chainHeight}.`);
      return chainHeight;
    }

    if (typeof lastSyncedBlockHeight !== 'number') {
      throw new Error(`Invalid last synced header height "${lastSyncedBlockHeight}"`);
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

  /**
   * Processing TXs during the historical sync
   */
  historicalTransactionsHandler() {

  }

  /**
   * Processing Merkle Blocks during the historical sync
   */
  historicalMerkleBlockHandler() {

  }

  /**
   * Processing new TXs during the continuous sync
   */
  newTransactionsHandler() {

  }

  /**
   * Processing new Merkle Blocks during the continuous sync
   */
  newMerkleBlockHandler() {

  }

  updateProgress() {
    // if (this.progressUpdateTimeout) {
    //   clearTimeout(this.progressUpdateTimeout);
    //   this.progressUpdateTimeout = null;
    // }
    //
    // const chainStore = this.storage.getChainStore(this.network.toString());
    //
    // const totalBlocksCount = chainStore.state.chainHeight + 1;
    // const syncedBlocksCount = this.lastSyncedBlockHeight + 1;
    // const transactionsCount = chainStore.state.transactions.size;
    // let progress = syncedBlocksCount / totalBlocksCount;
    // progress = Math.round(progress * 1000) / 10;
    // logger.debug(`[TransactionSyncStreamWorker] Historical fetch progress:
    // ${this.lastSyncedBlockHeight}/${chainStore.state.chainHeight}, ${progress}%`);
    // logger.debug(`[-------------------------->] TXs: ${transactionsCount}`);
    //
    // this.parentEvents.emit(EVENTS.TRANSACTIONS_SYNC_PROGRESS, {
    //   progress,
    //   syncedBlocksCount,
    //   totalBlocksCount,
    //   transactionsCount,
    // });
  }

  emitError(e) {
    this.parentEvents.emit('error', e);
  }
}

TransactionsSyncWorker.STATES = STATES;

module.exports = TransactionsSyncWorker;
