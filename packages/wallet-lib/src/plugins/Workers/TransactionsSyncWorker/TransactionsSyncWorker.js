const BlockHeadersProvider = require('@dashevo/dapi-client/lib/BlockHeadersProvider/BlockHeadersProvider');
const ReconnectableStream = require('@dashevo/dapi-client/lib/transport/ReconnectableStream');
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
        'importTransactions',
        'importInstantLock',
        'storage',
        'keyChainStore',
        'transport',
        'network',
        'walletId',
      ],
      ...options,
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
    this.instantLocksHandler = this.instantLocksHandler.bind(this);
    this.updateProgress = this.updateProgress.bind(this);

    this.syncState = STATES.IDLE;
  }

  inject(name, obj, allowSensitiveOperations = false) {
    super.inject(name, obj, allowSensitiveOperations);

    if (name === 'walletId') {
      this.logger = logger.getForWallet(this.walletId);
    }
  }

  async init() {
    if (!this.transactionsReader) {
      const createContinuousSyncStream = (bloomFilter, rangeOptions) => ReconnectableStream
        .create(
          this.transport.client.core.subscribeToTransactionsWithProofs,
          {
            maxRetriesOnError: -1,
          },
        )(
          bloomFilter,
          rangeOptions,
        );

      const createHistoricalSyncStream = (bloomFilter, rangeOptions) => {
        const { subscribeToTransactionsWithProofs } = this.transport.client.core;
        return subscribeToTransactionsWithProofs(
          bloomFilter,
          rangeOptions,
        );
      };

      this.transactionsReader = new TransactionsReader(
        {
          maxRetries: MAX_RETRIES,
          network: this.network,
          walletId: this.walletId,
        },
        createHistoricalSyncStream,
        createContinuousSyncStream,
      );
    }
  }

  async onStart() {
    await this.init();

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
      this.logger.silly(`[TransactionsSyncWorker] Start block height is equal to chain height ${chainHeight}, not syncing.`);
      this.syncCheckpoint = chainHeight;
      chainStore.updateLastSyncedBlockHeight(chainHeight);
      chainStore.pruneHeadersMetadata(chainHeight);
      this.storage.saveState();
      return;
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

    await this.transactionsReader
      .startHistoricalSync(startFrom, chainHeight, addresses);

    this.syncState = STATES.HISTORICAL_SYNC;

    let syncResult;
    try {
      syncResult = await historicalSyncPromise;
    } catch (e) {
      this.historicalTransactionsToVerify.clear();
      throw e;
    }

    chainStore.updateLastSyncedBlockHeight(chainHeight);
    this.updateProgress();

    if (!syncResult.stopped) {
      chainStore.pruneHeadersMetadata(chainHeight);
      this.syncCheckpoint = chainHeight;
    }
    this.storage.saveState();
    this.historicalTransactionsToVerify.clear();

    this.syncState = STATES.IDLE;
  }

  async execute() {
    await this.init();

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

    this.transactionsReader.on(
      TransactionsReader.EVENTS.INSTANT_LOCKS,
      this.instantLocksHandler,
    );

    this.transactionsReaderErrorHandler = (e) => {
      this.emitError(e);
      this.logger.debug('[TransactionsSyncWorker] Error handling continuous chain update', e);
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
        TransactionsReader.EVENTS.INSTANT_LOCKS,
        this.instantLocksHandler,
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
    this.logger.debug('[TransactionsSyncWorker] Stopping...');
    if (this.syncState === STATES.HISTORICAL_SYNC) {
      await this.transactionsReader.stopHistoricalSync();
    } else if (this.syncState === STATES.CONTINUOUS_SYNC) {
      await this.transactionsReader.stopContinuousSync();
    }

    this.transactionsReader = null;

    if (this.blockHeightChangedHandler) {
      this.parentEvents
        .removeListener(EVENTS.BLOCKHEIGHT_CHANGED, this.blockHeightChangedHandler);
      this.blockHeightChangedHandler = null;
    }

    this.syncState = STATES.IDLE;
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
      this.logger.debug(`[TransactionsSyncWorker] Wallet created from a new mnemonic. Sync from current chain height ${chainHeight}.`);
      return chainHeight;
    }

    if (typeof lastSyncedBlockHeight !== 'number') {
      throw new Error(`Invalid last synced header height "${lastSyncedBlockHeight}"`);
    }

    const skipBefore = parseInt(skipSynchronizationBeforeHeight, 10);

    if (skipBefore > lastSyncedBlockHeight) {
      this.logger.debug(`[TransactionsSyncWorker] UNSAFE option skipSynchronizationBeforeHeight is set to ${skipBefore}`);
      height = skipBefore;
    } else if (lastSyncedBlockHeight > -1) {
      this.logger.debug(`[TransactionsSyncWorker] Last synced block height is ${lastSyncedBlockHeight}`);
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

    const chainStore = this.storage.getDefaultChainStore();

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

    let addressesGenerated = [];
    if (this.historicalTransactionsToVerify.size) {
      const txHashesInTheBlock = merkleBlock
        .hashes.reduce((set, hashHex) => {
          const hash = Buffer.from(hashHex, 'hex').reverse();
          set.add(hash.toString('hex'));
          return set;
        }, new Set());

      const metadata = {
        blockHash: headerHash,
        height: headerHeight,
        time: new Date(headerTime * 1000),
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
          this.historicalTransactionsToVerify.delete(tx.hash);
        });
      } catch (e) {
        rejectMerkleBlock(e);
        return;
      }

      // TODO(spv): verify transactions against the merkle block

      if (transactionsWithMetadata.length) {
        ({ addressesGenerated } = this.importTransactions(transactionsWithMetadata));
      }
    }

    acceptMerkleBlock(headerHeight, addressesGenerated);

    this.syncCheckpoint = headerHeight;
    chainStore.updateLastSyncedBlockHeight(headerHeight);
    chainStore.pruneHeadersMetadata(headerHeight);
    this.storage.scheduleStateSave();
    this.scheduleProgressUpdate();
  }

  /**
   * @private
   * @param {NewTransactionsDataEventPayload} payload
   * Processing new TXs during the continuous sync
   */
  newTransactionsHandler(payload) {
    const { transactions, handleNewAddresses } = payload;

    if (!transactions.length) {
      throw new Error('No new transactions to process');
    }

    const chainStore = this.storage.getDefaultChainStore();
    const newTransactions = transactions
      .filter((tx) => !chainStore.state.transactions.has(tx.hash));

    if (newTransactions.length) {
      newTransactions.forEach((tx) => {
        this.historicalTransactionsToVerify.set(tx.hash, tx);
      });

      const { addressesGenerated } = this.importTransactions(
        newTransactions.map((tx) => [tx]),
      );

      handleNewAddresses(addressesGenerated);
    }
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
    const { headersMetadata, lastSyncedBlockHeight } = chainStore.state;
    const headerMetadata = headersMetadata.get(headerHash);

    // Header metadata was not found, subscribe to BLOCKHEIGHT_CHANGED event
    // in order to check one more time
    if (!headerMetadata) {
      this.logger.silly(`[TransactionsSyncWorker#newMerkleBlockHandler] header metadata not found for block "${merkleBlock.header.hash}". Waiting for chain height to change.`);
      if (this.blockHeightChangedHandler) {
        // This situation should not normally happen
        // because BlockHeadersSyncWorker should fire BLOCKHEIGHT_CHANGED either
        // before new MerkleBlock or immediately after, but set an error log just in case
        // TODO: probably remove after the debugging?
        this.logger.warn('[TransactionsSyncWorker] Block height changed handler is already set.');
        return;
      }

      this.blockHeightChangedHandler = () => {
        this.logger.silly(`[TransactionsSyncWorker#newMerkleBlockHandler] handled block height change event. Retry with merkle block ${merkleBlock.header.hash}`);
        this.newMerkleBlockHandler(payload);
        this.blockHeightChangedHandler = null;
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

    // TODO: revisit
    // Do nothing in case header height is the same as the last synced
    // this might happen when the stream reconnects from the same height.
    // Although we must consider the case with reorgs and invalidate transactions
    // from the headerHeight and later
    if (headerHeight <= lastSyncedBlockHeight) {
      return;
    }

    if (headerMetadata.time <= 0 || Number.isNaN(headerMetadata.time)) {
      rejectMerkleBlock(rejectMerkleBlock(Error(`Invalid header time: ${headerMetadata.time}`)));
      return;
    }

    const headerTime = new Date(headerMetadata.time * 1000);

    this.logger.debug('[TransactionsSyncWorker#newMerkleBlockHandler] New merkle block received', {
      hash: merkleBlock.header.hash,
      height: headerHeight,
      time: headerTime,
    });

    let $transactionsFound = 0;
    if (this.historicalTransactionsToVerify.size) {
      const txHashesInTheBlock = getTxHashesFromMerkleBlock(merkleBlock);

      // TODO: verify merkle block in SPV

      const metadata = {
        blockHash: headerHash,
        height: headerHeight,
        time: headerTime,
        instantLocked: false, // TBD
        chainLocked: false, // TBD
      };

      const transactionsWithMetadata = [];

      // Traverse through all transactions to verify and re-import ones having metadata
      this.historicalTransactionsToVerify.forEach((tx) => {
        if (txHashesInTheBlock.has(tx.hash)) {
          transactionsWithMetadata.push([tx, metadata]);
          this.historicalTransactionsToVerify.delete(tx.hash);
        }
      });

      if (transactionsWithMetadata.length) {
        this.importTransactions(transactionsWithMetadata);
        transactionsWithMetadata.forEach(([tx]) => {
          this.parentEvents.emit(EVENTS.CONFIRMED_TRANSACTION, tx);
        });
        $transactionsFound = transactionsWithMetadata.length;
      }
    }

    acceptMerkleBlock(headerHeight);
    this.syncCheckpoint = headerHeight;
    chainStore.updateLastSyncedBlockHeight(headerHeight);
    chainStore.pruneHeadersMetadata(headerHeight);
    this.storage.scheduleStateSave();

    this.logger.debug(`[TransactionsSyncWorker#newMerkleBlockHandler] ${$transactionsFound} txs found, ${this.historicalTransactionsToVerify.size} pending to be verified.`);
  }

  /**
   * @private
   * Processing is locks during the continuous sync
   * @param {InstantLock[]} instantLocks
   */
  instantLocksHandler(instantLocks) {
    // TODO: perform IS locks verification
    instantLocks.forEach((instantLock) => {
      this.importInstantLock(instantLock);
    });
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
  // eslint-disable-next-line
  updateProgress() {
    if (this.progressUpdateTimeout) {
      clearTimeout(this.progressUpdateTimeout);
      this.progressUpdateTimeout = null;
    }

    const chainStore = this.storage.getDefaultChainStore();

    const { chainHeight, lastSyncedBlockHeight, transactions } = chainStore.state;
    const totalBlocksCount = chainHeight + 1;
    const syncedBlocksCount = lastSyncedBlockHeight + 1;
    const transactionsCount = transactions.size;
    let progress = syncedBlocksCount / totalBlocksCount;
    progress = Math.round(progress * 1000) / 10;
    this.logger.debug(`[TransactionSyncStreamWorker] Historical fetch progress: ${lastSyncedBlockHeight}/${chainStore.state.chainHeight}, ${progress}%`);
    this.logger.debug(`[-------------------------->] TXs: ${transactionsCount}`);

    this.parentEvents.emit(EVENTS.TRANSACTIONS_SYNC_PROGRESS, {
      progress,
      syncedBlocksCount,
      totalBlocksCount,
      transactionsCount,
    });
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
