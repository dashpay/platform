const BlockHeadersProvider = require('@dashevo/dapi-client/lib/BlockHeadersProvider/BlockHeadersProvider');
const Worker = require('../../Worker');
const logger = require('../../../logger');
const TransactionsReader = require('./TransactionsReader');
const { getAddressesToSync, getTxHashesFromMerkleBlock } = require('./utils');
const EVENTS = require('../../../EVENTS');

const STATES = {
  IDLE: 'STATE_IDLE',
  HISTORICAL_SYNC: 'STATE_HISTORICAL_SYNC',
  CONTINUOUS_SYNC: 'STATE_CONTINUOUS_SYNC',
};

const MAX_RETRIES = 10;
const PROGRESS_UPDATE_INTERVAL = 1000;

/**
 * @typedef MerkleBlockDataEventPayload
 * @property {MerkleBlock} merkleBlock
 * @property {Function} acceptMerkleBlock
 * @property {Function} rejectMerkleBlock
 */

/**
 * @typedef NewTransactionsDataEventPayload
 * @property {Transaction[]} transactions
 * @property {Function} appendAddresses
 */

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
    this.progressUpdateTimeout = null;

    /**
     * Pool of historical transactions to be verified and imported
     * @type {Map<string, Transaction>}
     */
    this.historicalTransactionsToVerify = new Map();

    this.transactionsReaderErrorHandler = null;
    this.transactionsReaderStoppedHandler = null;
    this.historicalDataObtainedHandler = null;

    this.blockHeightChangedHandler = null;

    this.historicalTransactionsHandler = this.historicalTransactionsHandler.bind(this);
    this.historicalMerkleBlockHandler = this.historicalMerkleBlockHandler.bind(this);
    this.newTransactionsHandler = this.newTransactionsHandler.bind(this);
    this.newMerkleBlockHandler = this.newMerkleBlockHandler.bind(this);

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
      // chainStore.clearHeadersMetadata();
      return;
    }

    const {
      skipSynchronization,
    } = (this.storage.application.syncOptions || {});

    if (skipSynchronization) {
      logger.debug(`[TransactionSyncStreamWorker] Wallet created from a new mnemonic. Sync from the current chain height ${chainHeight}`);
      chainStore.updateLastSyncedBlockHeight(chainHeight);
      this.syncCheckpoint = chainHeight;
      // chainStore.clearHeadersMetadata();
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

        this.historicalTransactionsToVerify.clear();

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

        if (this.historicalTransactionsToVerify.size > 0) {
          reject(new Error('Historical data obtained but there are still transactions to verify'));
        } else {
          resolve({
            stopped: false,
          });
        }
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

    // Sync up to chainHeight -1 to avoid overlapping with continuous sync starting point
    await this.transactionsReader
      .startHistoricalSync(startFrom, chainHeight - 1, addresses);

    this.syncState = STATES.HISTORICAL_SYNC;

    let syncResult;
    try {
      syncResult = await historicalSyncPromise;
    } catch (e) {
      this.historicalTransactionsToVerify.clear();
      throw e;
    }

    this.updateProgress();
    this.storage.saveState();

    if (!syncResult.stopped) {
      // TODO(spv): rework to clear only metadata that was actually used
      // chainStore.clearHeadersMetadata();
      this.syncCheckpoint = chainHeight;
    }

    this.historicalTransactionsToVerify.clear();

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

    if (this.blockHeightChangedHandler) {
      this.parentEvents
        .removeListener(EVENTS.BLOCKHEIGHT_CHANGED, this.blockHeightChangedHandler);
      this.blockHeightChangedHandler = null;
    }
  }

  /**
   * @private
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
   * @private
   * Processing TXs during the historical sync
   * @param {Transaction[]} transactions
   */
  historicalTransactionsHandler(transactions) {
    if (!transactions.length) {
      throw new Error('No transactions to process');
    }

    transactions.forEach((tx) => {
      this.historicalTransactionsToVerify.set(tx.hash, tx);
    });
  }

  /**
   * @private
   * Processing Merkle Blocks during the historical sync
   * @param {MerkleBlockDataEventPayload} payload
   */
  historicalMerkleBlockHandler(payload) {
    const { merkleBlock, acceptMerkleBlock, rejectMerkleBlock } = payload;
    if (!this.historicalTransactionsToVerify.size) {
      rejectMerkleBlock(new Error(`No transactions to verify for merkle block ${merkleBlock.header.hash}`));
      return;
    }
    const chainStore = this.storage.getDefaultChainStore();

    const txHashesInTheBlock = merkleBlock
      .hashes.reduce((set, hashHex) => {
        const hash = Buffer.from(hashHex, 'hex').reverse();
        set.add(hash.toString('hex'));
        return set;
      }, new Set());

    const headerHash = merkleBlock.header.hash;
    const headerMetadata = chainStore.state.headersMetadata.get(headerHash);

    if (!headerMetadata) {
      rejectMerkleBlock(
        new Error('Header metadata was not found during the merkle block processing'),
      );
      return;
    }

    const headerHeight = headerMetadata.height;
    const headerTime = headerMetadata.time;

    if (headerHeight < 0 || Number.isNaN(headerHeight)) {
      rejectMerkleBlock(Error(`Invalid header height: ${headerHeight}`));
      return;
    }

    if (headerTime <= 0 || Number.isNaN(headerTime)) {
      rejectMerkleBlock(rejectMerkleBlock(Error(`Invalid header time: ${headerTime}`)));
      return;
    }

    const metadata = {
      blockHash: headerHash,
      height: headerHeight,
      time: new Date(headerTime * 1e3),
      instantLocked: false, // TBD
      chainLocked: false, // TBD
    };

    const transactionsWithMetadata = [];

    try {
      this.historicalTransactionsToVerify.forEach((tx) => {
        if (!txHashesInTheBlock.has(tx.hash)) {
          throw new Error(`Transaction ${tx.hash} was not found in merkle block ${headerHash}`);
        }
        transactionsWithMetadata.push([tx, metadata]);
        delete this.historicalTransactionsToVerify[tx.hash];
      });
    } catch (e) {
      rejectMerkleBlock(e);
      return;
    }

    // TODO(spv): verify transactions against the merkle block

    let addressesGenerated = [];
    if (transactionsWithMetadata.length) {
      ({ addressesGenerated } = this.importTransactions(transactionsWithMetadata));
    }

    acceptMerkleBlock(headerHeight, addressesGenerated);

    this.syncCheckpoint = headerHeight;
    chainStore.updateLastSyncedBlockHeight(headerHeight);
    this.storage.scheduleStateSave();
    this.scheduleProgressUpdate();
  }

  /**
   * @private
   * @param {NewTransactionsDataEventPayload} payload
   * Processing new TXs during the continuous sync
   */
  newTransactionsHandler(payload) {
    const { transactions, appendAddresses } = payload;

    if (!transactions.length) {
      throw new Error('No new transactions to process');
    }

    transactions.forEach((tx) => {
      this.historicalTransactionsToVerify.set(tx.hash, tx);
    });

    const { addressesGenerated } = this.importTransactions(transactions);

    appendAddresses(addressesGenerated);
  }

  /**
   * @private
   * Processing Merkle Blocks during the historical sync
   * @param {MerkleBlockDataEventPayload} payload
   */
  newMerkleBlockHandler(payload) {
    const { merkleBlock, acceptMerkleBlock, rejectMerkleBlock } = payload;

    const chainStore = this.storage.getDefaultChainStore();

    const headerHash = merkleBlock.header.hash;
    const headerMetadata = chainStore.state.headersMetadata.get(headerHash);

    // Header metadata was not found, subscribe to BLOCKHEIGHT_CHANGED event
    // in order to check one more time
    if (!headerMetadata) {
      if (this.blockHeightChangedHandler) {
        // This situation should not normally happen
        // because BlockHeadersSyncWorker should fire BLOCKHEIGHT_CHANGED either
        // before new MerkleBlock or immediately after, but set an error log just in case
        // TODO: probably remove after the debugging?
        logger.error('[TransactionsSyncWorker] Block height changed handler is already set.');
        return;
      }

      this.blockHeightChangedHandler = () => {
        this.newMerkleBlockHandler(payload);
      };

      this.parentEvents.once(
        EVENTS.BLOCKHEIGHT_CHANGED,
        this.blockHeightChangedHandler,
      );

      return;
    }

    const headerHeight = headerMetadata.height;
    if (headerHeight < 0 || Number.isNaN(headerHeight)) {
      rejectMerkleBlock(Error(`Invalid header height: ${headerHeight}`));
      return;
    }

    const headerTime = headerMetadata.time;
    if (headerTime <= 0 || Number.isNaN(headerTime)) {
      rejectMerkleBlock(rejectMerkleBlock(Error(`Invalid header time: ${headerTime}`)));
      return;
    }

    if (this.historicalTransactionsToVerify.size) {
      const txHashesInTheBlock = getTxHashesFromMerkleBlock(merkleBlock);

      // TODO: verify merkle block in SPV

      const metadata = {
        blockHash: headerHash,
        height: headerHeight,
        time: new Date(headerTime * 1e3),
        instantLocked: false, // TBD
        chainLocked: false, // TBD
      };

      const transactionsWithMetadata = [];

      // Traverse through all transactions to verify and re-import ones having metadata
      this.historicalTransactionsToVerify.forEach((tx) => {
        if (txHashesInTheBlock.has(tx.hash)) {
          transactionsWithMetadata.push([tx, metadata]);
          delete this.historicalTransactionsToVerify[tx.hash];
        }
      });

      if (transactionsWithMetadata.length) {
        this.importTransactions(transactionsWithMetadata);
        transactionsWithMetadata.forEach(([tx]) => {
          this.parentEvents.emit(EVENTS.CONFIRMED_TRANSACTION, tx);
        });
      }
    }

    acceptMerkleBlock(headerHeight);
    this.syncCheckpoint = headerHeight;
    chainStore.updateLastSyncedBlockHeight(headerHeight);
    this.storage.scheduleStateSave();
  }

  /**
   * @private
   */
  scheduleProgressUpdate() {
    if (!this.progressUpdateTimeout) {
      this.progressUpdateTimeout = setTimeout(this.updateProgress, PROGRESS_UPDATE_INTERVAL);
    }
  }

  /**
   * @private
   */
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

  /**
   * @private
   * @param {Error} e
   */
  emitError(e) {
    this.parentEvents.emit('error', e);
  }
}

TransactionsSyncWorker.STATES = STATES;

module.exports = TransactionsSyncWorker;
