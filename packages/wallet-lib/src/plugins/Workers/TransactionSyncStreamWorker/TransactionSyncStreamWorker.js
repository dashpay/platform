const {
  Transaction, MerkleBlock, InstantLock,
} = require('@dashevo/dashcore-lib');
const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const sleep = require('../../../utils/sleep');

const Worker = require('../../Worker');
const isBrowser = require('../../../utils/isBrowser');

const logger = require('../../../logger');

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
        'importBlockHeader',
        'importInstantLock',
        'storage',
        'transport',
        'walletId',
        'getAddress',
        'network',
        'index',
        'BIP44PATH',
        'walletType',
      ],
      ...options,
    });

    this.syncIncomingTransactions = false;
    this.stream = null;
    this.incomingSyncPromise = null;
    this.pendingRequest = {};
    this.delayedRequests = {};
  }

  /**
   * Filter transaction based on the address list
   * @param {Transaction[]} transactions
   * @param {string[]} addressList
   * @param {string} network
   */
  static filterWalletTransactions(transactions, addressList, network) {
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
    } = (this.storage.store.syncOptions || {});

    if (skipSynchronization) {
      logger.debug('TransactionSyncStreamWorker - Wallet created from a new mnemonic. Sync from the best block height.');
      const bestBlockHeight = this.storage.store.chains[this.network.toString()].blockHeight;
      this.setLastSyncedBlockHeight(bestBlockHeight);
      return;
    }

    if (skipSynchronizationBeforeHeight) {
      this.setLastSyncedBlockHeight(
        skipSynchronizationBeforeHeight,
      );
    }

    // We first need to sync up initial historical transactions
    await this.startHistoricalSync(this.network);
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

    // noinspection ES6MissingAwait
    this.incomingSyncPromise = this.startIncomingSync();
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
    const { walletId } = this;
    const accountsStore = this.storage.store.wallets[walletId].accounts;

    const accountStore = ([WALLET_TYPES.HDWALLET, WALLET_TYPES.HDPUBLIC].includes(this.walletType))
      ? accountsStore[this.BIP44PATH.toString()]
      : accountsStore[this.index.toString()];

    accountStore.blockHash = hash;

    return accountStore.blockHash;
  }

  getLastSyncedBlockHash() {
    const { walletId } = this;
    const accountsStore = this.storage.store.wallets[walletId].accounts;

    const { blockHash } = ([WALLET_TYPES.HDWALLET, WALLET_TYPES.HDPUBLIC].includes(this.walletType))
      ? accountsStore[this.BIP44PATH.toString()]
      : accountsStore[this.index.toString()];

    return blockHash;
  }
}

TransactionSyncStreamWorker.prototype.getAddressesToSync = require('./methods/getAddressesToSync');
TransactionSyncStreamWorker.prototype.getBestBlockHeightFromTransport = require('./methods/getBestBlockHeight');
TransactionSyncStreamWorker.prototype.setLastSyncedBlockHeight = require('./methods/setLastSyncedBlockHeight');
TransactionSyncStreamWorker.prototype.getLastSyncedBlockHeight = require('./methods/getLastSyncedBlockHeight');
TransactionSyncStreamWorker.prototype.startHistoricalSync = require('./methods/startHistoricalSync');
TransactionSyncStreamWorker.prototype.handleTransactionFromStream = require('./methods/handleTransactionFromStream');
TransactionSyncStreamWorker.prototype.processChunks = require('./methods/processChunks');
TransactionSyncStreamWorker.prototype.startIncomingSync = require('./methods/startIncomingSync');
TransactionSyncStreamWorker.prototype.syncUpToTheGapLimit = require('./methods/syncUpToTheGapLimit');

module.exports = TransactionSyncStreamWorker;
