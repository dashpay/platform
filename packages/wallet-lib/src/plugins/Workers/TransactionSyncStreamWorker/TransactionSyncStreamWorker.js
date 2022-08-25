const {
  Transaction, MerkleBlock, InstantLock,
} = require('@dashevo/dashcore-lib');
const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const sleep = require('../../../utils/sleep');

const Worker = require('../../Worker');
const isBrowser = require('../../../utils/isBrowser');

const logger = require('../../../logger');
const ChainSyncMediator = require('../../../types/Wallet/ChainSyncMediator');
const EVENTS = require('../../../EVENTS');

const PROGRESS_UPDATE_INTERVAL = 1000;

class TransactionSyncStreamWorker extends Worker {
  constructor(options) {
    super({
      name: 'TransactionSyncStreamWorker',
      executeOnStart: true,
      firstExecutionRequired: true,
      awaitOnInjection: true,
      workerIntervalTime: 0,
      gapLimit: 10,
      dependencies: [
        'importTransactions',
        'importInstantLock',
        'storage',
        'keyChainStore',
        'transport',
        'walletId',
        'getAddress',
        'network',
        'index',
        'BIP44PATH',
        'walletType',
        'chainSyncMediator',
      ],
      ...options,
    });

    // TODO: cleanup
    this.syncIncomingTransactions = false;
    this.stream = null;
    this.incomingSyncPromise = null;
    this.pendingRequest = {};
    this.delayedRequests = {};
    this.lastSyncedBlockHeight = -1;
    this.progressUpdateTimeout = null;

    /**
     * Indicates that TX stream has to be re-created
     * with the new addresses for bloom filter after the chain height has been updated
     * @type {boolean}
     */
    this.reconnectOnNewBlock = false;

    /**
     * Pool of transactions pending to be verified
     * @type {{}}
     */
    this.transactionsToVerify = {};

    this.scheduleProgressUpdate = this.scheduleProgressUpdate.bind(this);
    this.updateProgress = this.updateProgress.bind(this);
    this.handleNewBlock = this.handleNewBlock.bind(this);
  }

  /**
   * Filter transaction based on the address list
   * @param {Transaction[]} transactions
   * @param {string[]} addressList
   * @param {string} network
   */
  static filterAddressesTransactions(transactions, addressList, network) {
    const spentOutputs = [];
    const unspentOutputs = [];
    const filteredTransactions = transactions.filter((tx) => {
      let isWalletTransaction = false;

      tx.inputs.forEach((input) => {
        if (input.script) {
          const addr = input.script.toAddress(network).toString();
          if (addressList.includes(addr)) {
            spentOutputs.push(input);
            isWalletTransaction = true;
          }
        }
      });

      tx.outputs.forEach((output) => {
        const addr = output.script.toAddress(network).toString();
        if (addressList.includes(addr)) {
          unspentOutputs.push(output);
          isWalletTransaction = true;
        }
      });

      return isWalletTransaction;
    });

    return {
      transactions: filteredTransactions,
      spentOutputs,
      unspentOutputs,
    };
  }

  /**
   *
   * @param {TransactionsWithProofsResponse} response
   * @return {[]}
   */
  static getMerkleBlockFromStreamResponse(response) {
    let merkleBlock = null;
    const rawMerkleBlock = response.getRawMerkleBlock();
    if (rawMerkleBlock) {
      merkleBlock = new MerkleBlock(Buffer.from(rawMerkleBlock));
    }
    return merkleBlock;
  }

  /**
   *
   * @param response
   * @return {[]}
   */
  static getTransactionListFromStreamResponse(response) {
    let walletTransactions = [];
    const transactions = response.getRawTransactions();

    if (transactions) {
      walletTransactions = transactions
        .getTransactionsList()
        .map((rawTransaction) => new Transaction(Buffer.from(rawTransaction)));
    }

    return walletTransactions;
  }

  static getInstantSendLocksFromResponse(response) {
    let walletTransactions = [];
    const instantSendLockMessages = response.getInstantSendLockMessages();

    if (instantSendLockMessages) {
      walletTransactions = instantSendLockMessages
        .getMessagesList()
        .map((instantSendLock) => new InstantLock(Buffer.from(instantSendLock)));
    }

    return walletTransactions;
  }

  async onStart() {
    // Using sync options here to avoid
    // situation when plugin is injected directly
    // instead of usual injection process
    const {
      skipSynchronizationBeforeHeight,
      skipSynchronization,
    } = (this.storage.application.syncOptions || {});

    const bestBlockHeight = this.storage.getChainStore(this.network.toString()).state.blockHeight;
    if (skipSynchronization) {
      logger.debug('TransactionSyncStreamWorker - Wallet created from a new mnemonic. Sync from the best block height.');
      // TODO: probably this check has to go to a completely different place (ChainPlugin?)
      this.setLastSyncedBlockHeight(bestBlockHeight);
      return;
    }

    const { lastKnownBlock } = this.storage.getWalletStore(this.walletId).state;
    const skipSyncBefore = typeof skipSynchronizationBeforeHeight === 'number'
      ? skipSynchronizationBeforeHeight
      : parseInt(skipSynchronizationBeforeHeight, 10);

    if (skipSyncBefore > lastKnownBlock.height) {
      this.setLastSyncedBlockHeight(
        // TODO: shouldn't be skipSyncBefore instead?
        skipSynchronizationBeforeHeight,
      );
    } else if (lastKnownBlock.height !== -1) {
      this.setLastSyncedBlockHeight(lastKnownBlock.height);
    }

    this.chainSyncMediator.state = ChainSyncMediator.STATES.HISTORICAL_SYNC;
    // We first need to sync up initial historical transactions
    await this.startHistoricalSync(this.network);
    this.setLastSyncedBlockHeight(bestBlockHeight, true);
    this.updateProgress();
    await this.storage.saveState();
    // TODO: Purge headers metadata from storage
    // TODO: for some reason headers still continue arriving after everything is completed
  }

  /**
   * This is executed only once on start up.
   * So we will maintain our ongoing stream during the whole execution of the wallet
   *
   * @returns {Promise<void>}
   */
  async execute() {
    this.syncIncomingTransactions = true;
    // We shouldn't block workers execution process with transaction syncing
    // it should proceed in background

    this.chainSyncMediator.state = ChainSyncMediator.STATES.CONTINUOUS_SYNC;
    // noinspection ES6MissingAwait
    this.incomingSyncPromise = this.startIncomingSync().catch((e) => {
      logger.error('Error syncing incoming transactions', e);
      this.parentEvents.emit('error', e);
    });

    this.parentEvents.on(EVENTS.BLOCK, this.handleNewBlock);
  }

  handleNewBlock(block, height) {
    const metadata = {
      height,
      blockHash: block.hash,
      instantLocked: false, // TBD,
      chainLocked: false, // TBD
    };

    // TODO: also search for transactions to verify
    // inside the existing headers metadata
    const transactionsWithMetadata = [];
    block.transactions.forEach((tx) => {
      if (this.transactionsToVerify[tx.hash]) {
        transactionsWithMetadata.push([tx, metadata]);
        delete transactionsWithMetadata[tx.hash];
      }
    });

    this.importTransactions(transactionsWithMetadata);

    this.setLastSyncedBlockHeight(height, true);

    if (this.reconnectOnNewBlock) {
      this.reconnectOnNewBlock = false;
      this.reconnectToStream()
        .catch((e) => {
          this.parentEvents.emit('error', e);
        });
    }
  }

  /**
   * @param {Object} [options]
   * @param {Boolean} [options.force=false]
   * @param {Boolean} [options.reason]
   *
   * @returns {Promise<boolean>}
   */
  async onStop(options = {}) {
    // in case of disconnect we don't need to wait until the wallet complete the sync process
    if (options.force) {
      this.pendingRequest = {};
    }

    // Sync, will require transaction and their blockHeader to be fetched before resolving.
    // As await onStop() is a way to wait for execution before continuing,
    // this ensure onStop will properly let the plugin to warn about all
    // completion of pending request.
    if (Object.keys(this.pendingRequest).length !== 0) {
      await sleep(200);

      return this.onStop();
    }

    this.syncIncomingTransactions = false;

    if (isBrowser()) {
      // Under browser environment, grpc-web doesn't call error and end events
      // so we call it by ourselves
      if (this.stream) {
        return new Promise((resolve) => setImmediate(() => {
          if (this.stream) {
            this.stream.cancel();

            const error = new GrpcError(GrpcErrorCodes.CANCELLED, 'Cancelled on client');
            // call onError events
            this.stream.f.forEach((func) => func(error));

            // call onEnd events
            this.stream.c.forEach((func) => func());

            this.stream = null;
          }

          resolve(true);
        }));
      }
    }

    this.parentEvents.removeListener(EVENTS.BLOCK, this.handleNewBlock);

    // Wrapping `cancel` in `setImmediate` due to bug with double-free
    // explained here (https://github.com/grpc/grpc-node/issues/1652)
    // and here (https://github.com/nodejs/node/issues/38964)
    return new Promise((resolve) => setImmediate(() => {
      if (this.stream) {
        this.stream.cancel();
        // When calling stream.cancel(), the stream will emit 'error' event
        // with the code 'CANCELLED'.
        // There are two cases when this happens: when the gap limit is filled
        // and syncToTheGapLimit and the stream needs to be restarted with new parameters,
        // and here, when stopping the worker.
        // The code in stream worker distinguishes whether it need to reconnect or not by the fact
        // that the old stream object is present or not. When it is set to null, it won't try to
        // reconnect to the stream.
        this.stream = null;
      }
      resolve(true);
    }));
  }

  setLastSyncedBlockHash(hash) {
    const applicationStore = this.storage.application;
    applicationStore.blockHash = hash;
    return applicationStore.blockHash;
  }

  getLastSyncedBlockHash() {
    const { blockHash } = this.storage.application;

    return blockHash;
  }

  updateProgress() {
    if (this.progressUpdateTimeout) {
      clearTimeout(this.progressUpdateTimeout);
      this.progressUpdateTimeout = null;
    }

    const chainStore = this.storage.getChainStore(this.network.toString());

    const totalBlocksCount = chainStore.state.blockHeight + 1;
    const syncedBlocksCount = this.lastSyncedBlockHeight + 1;
    const transactionsCount = chainStore.state.transactions.size;
    let progress = syncedBlocksCount / totalBlocksCount;
    progress = Math.round(progress * 1000) / 10;
    logger.debug(`[TransactionSynsStreamWorker] Historical fetch progress: ${this.lastSyncedBlockHeight}/${chainStore.chainHeight}, ${progress}%`);
    // TODO: add tests
    this.parentEvents.emit(EVENTS.TRANSACTIONS_SYNC_PROGRESS, {
      progress,
      syncedBlocksCount,
      totalBlocksCount,
      transactionsCount,
    });
  }

  scheduleProgressUpdate() {
    if (!this.progressUpdateTimeout) {
      this.progressUpdateTimeout = setTimeout(this.updateProgress, PROGRESS_UPDATE_INTERVAL);
    }
  }

  // eslint-disable-next-line class-methods-use-this
  removeStreamListeners(stream) {
    stream.removeAllListeners('data');
    stream.removeAllListeners('error');
    stream.removeAllListeners('end');
  }

  /**
   * Re-creates current stream,
   * so that new one could be re-created with more addresses in bloom filter
   * @returns {Promise<void>}
   */
  async reconnectToStream() {
    logger.silly('TransactionSyncStreamWorker - end stream - new addresses generated');

    if (isBrowser()) {
      // Under browser environment, grpc-web doesn't call error and end events
      // so we call it by ourselves
      await new Promise((resolveCancel) => setImmediate(() => {
        this.stream.cancel();
        const error = new GrpcError(GrpcErrorCodes.CANCELLED, 'Cancelled on client');

        // call onError events
        this.stream.f.forEach((func) => func(error));

        // call onEnd events
        this.stream.c.forEach((func) => func());
        resolveCancel();
      }));
    } else {
      // If there are some new addresses being imported
      // to the storage, that mean that we hit the gap limit
      // and we need to update the bloom filter with new addresses,
      // i.e. we need to open another stream with a bloom filter
      // that contains new addresses.

      // DO not setting null this.stream allow to know we
      // need to reset our stream (as we pass along the error)
      // Wrapping `cancel` in `setImmediate` due to bug with double-free
      // explained here (https://github.com/grpc/grpc-node/issues/1652)
      // and here (https://github.com/nodejs/node/issues/38964)
      await new Promise((resolveCancel) => setImmediate(() => {
        this.stream.cancel();
        resolveCancel();
      }));
    }
  }
}

TransactionSyncStreamWorker.prototype.getAddressesToSync = require('./methods/getAddressesToSync');
TransactionSyncStreamWorker.prototype.getBestBlockHeightFromTransport = require('./methods/getBestBlockHeight');
TransactionSyncStreamWorker.prototype.setLastSyncedBlockHeight = require('./methods/setLastSyncedBlockHeight');
TransactionSyncStreamWorker.prototype.getLastSyncedBlockHeight = require('./methods/getLastSyncedBlockHeight');
TransactionSyncStreamWorker.prototype.startHistoricalSync = require('./methods/startHistoricalSync');
TransactionSyncStreamWorker.prototype.processChunks = require('./methods/processChunks');
TransactionSyncStreamWorker.prototype.startIncomingSync = require('./methods/startIncomingSync');
TransactionSyncStreamWorker.prototype.syncUpToTheGapLimit = require('./methods/syncUpToTheGapLimit');

module.exports = TransactionSyncStreamWorker;
