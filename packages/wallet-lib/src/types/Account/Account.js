const _ = require('lodash');
const EventEmitter = require('events');
const logger = require('../../logger');
const { WALLET_TYPES, BIP44_ADDRESS_GAP } = require('../../CONSTANTS');
const { is } = require('../../utils');
const EVENTS = require('../../EVENTS');
const Wallet = require('../Wallet/Wallet');
const { simpleDescendingAccumulator } = require('../../utils/coinSelections/strategies');
const {
  TxMetadataTimeoutError,
  InstantLockTimeoutError,
} = require('../../errors');

function getNextUnusedAccountIndexForWallet(wallet) {
  if (wallet && wallet.accounts) {
    if (!wallet.accounts.length) return 0;

    const indexes = wallet.accounts.reduce((acc, curr) => {
      acc.push(curr.index);
      return acc;
    }, []).sort();
    let index;
    for (let i = 0; i <= indexes[indexes.length - 1] + 1; i += 1) {
      if (!indexes.includes(i)) {
        index = i;
        break;
      }
    }
    return index;
  }
  throw new Error('An account is attached to a wallet that has not been provided to the account constructor.');
}

const defaultOptions = {
  network: 'testnet',
  cacheTx: true,
  cacheBlockHeaders: true,
  allowSensitiveOperations: false,
  plugins: [],
  injectDefaultPlugins: true,
  debug: false,
  strategy: simpleDescendingAccumulator,
};

/* eslint-disable no-underscore-dangle */
const _initializeAccount = require('./_initializeAccount');
const _addAccountToWallet = require('./_addAccountToWallet');
const _loadStrategy = require('./_loadStrategy');

const getNetwork = require('./_getNetwork');
const getBIP44Path = require('./_getBIP44Path');

class Account extends EventEmitter {
  constructor(wallet, opts = defaultOptions) {
    super();
    if (!wallet || wallet.constructor.name !== Wallet.name) throw new Error('Expected wallet to be passed as param');
    if (!_.has(wallet, 'walletId')) throw new Error('Missing walletID to create an account');
    this.walletId = wallet.walletId;

    logger.debug(`Loading up wallet ${this.walletId}`);

    this.identities = wallet.identities;
    this.chainSyncMediator = wallet.chainSyncMediator;

    this.state = {
      isInitialized: false,
      isReady: false,
      isDisconnecting: false,
    };
    this.injectDefaultPlugins = _.has(opts, 'injectDefaultPlugins') ? opts.injectDefaultPlugins : defaultOptions.injectDefaultPlugins;
    this.allowSensitiveOperations = _.has(opts, 'allowSensitiveOperations') ? opts.allowSensitiveOperations : defaultOptions.allowSensitiveOperations;
    this.debug = _.has(opts, 'debug') ? opts.debug : defaultOptions.debug;
    // if (this.debug) process.env.LOG_LEVEL = 'debug';

    this.waitForInstantLockTimeout = wallet.waitForInstantLockTimeout;
    this.waitForTxMetadataTimeout = wallet.waitForTxMetadataTimeout;

    this.walletType = wallet.walletType;
    this.offlineMode = wallet.offlineMode;

    this.index = _.has(opts, 'index') ? opts.index : getNextUnusedAccountIndexForWallet(wallet);
    this.strategy = _loadStrategy(_.has(opts, 'strategy') ? opts.strategy : defaultOptions.strategy);
    this.network = getNetwork(wallet.network).toString();
    this.BIP44PATH = getBIP44Path(this.network, this.index);

    this.transactions = {};

    this.label = (opts && opts.label && is.string(opts.label)) ? opts.label : null;

    // Forward async error events to wallet allowing catching during initial sync
    this.on('error', (error, errorContext) => wallet.emit('error', error, {
      ...errorContext,
      accountIndex: this.index,
      network: this.network,
      label: this.label,
    }));

    // If transport is null or invalid, we won't try to fetch anything
    this.transport = wallet.transport;

    this.storage = wallet.storage;

    // Forward all storage event
    this.storage.on(EVENTS.CONFIGURED, (ev) => this.emit(ev.type, ev));
    this.storage.on(EVENTS.REHYDRATE_STATE_FAILED, (ev) => this.emit(ev.type, ev));
    this.storage.on(EVENTS.REHYDRATE_STATE_SUCCESS, (ev) => this.emit(ev.type, ev));
    this.storage.on(EVENTS.FETCHED_CONFIRMED_TRANSACTION, (ev) => this.emit(ev.type, ev));
    this.storage.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, (ev) => this.emit(ev.type, ev));
    this.storage.on(EVENTS.CONFIRMED_BALANCE_CHANGED, (ev) => this.emit(ev.type, ev));
    this.storage.on(EVENTS.TX_METADATA, (ev) => {
      this.emit(`${ev.type}:${ev.payload.hash}`, ev.payload.metadata);
    });
    this.storage.on(EVENTS.BLOCKHEADER, (ev) => this.emit(ev.type, ev));
    this.storage.on(EVENTS.BLOCKHEIGHT_CHANGED, (ev) => this.emit(ev.type, ev));
    this.storage.on(EVENTS.BLOCK, (ev) => this.emit(ev.type, ev));

    if (this.debug) {
      this.emit = (...args) => {
        const { type } = args[1];
        const payload = JSON.stringify(args[1].payload);
        logger.debug(`${this.walletId}:${this.index} - Emitted event ${type} - ${payload} `);
        super.emit(...args);
      };
    }
    switch (this.walletType) {
      case WALLET_TYPES.HDWALLET:
        this.accountPath = getBIP44Path(this.network, this.index);
        break;
      case WALLET_TYPES.HDPUBLIC:
      case WALLET_TYPES.PRIVATEKEY:
      case WALLET_TYPES.PUBLICKEY:
      case WALLET_TYPES.ADDRESS:
      case WALLET_TYPES.SINGLE_ADDRESS:
        this.accountPath = 'm/0';
        break;
      default:
        throw new Error(`Invalid wallet type ${this.walletType}`);
    }

    this.storage
      .getWalletStore(this.walletId)
      .createPathState(this.accountPath);

    let keyChainStorePath = this.index;
    const keyChainStoreOpts = {};

    switch (this.walletType) {
      case WALLET_TYPES.HDPUBLIC:
        keyChainStorePath = this.accountPath;
        keyChainStoreOpts.lookAheadOpts = {
          paths: {
            'm/0': BIP44_ADDRESS_GAP,
          },
        };
        break;
      case WALLET_TYPES.HDWALLET:
      case WALLET_TYPES.HDPRIVATE:
        keyChainStorePath = this.BIP44PATH;
        keyChainStoreOpts.lookAheadOpts = {
          paths: {
            'm/0': BIP44_ADDRESS_GAP,
            'm/1': BIP44_ADDRESS_GAP,
          },
        };
        break;
      default:
        break;
    }

    this.keyChainStore = wallet
      .keyChainStore
      .makeChildKeyChainStore(keyChainStorePath, keyChainStoreOpts);

    // This forces keychainStore to set to issued key what is already its masterkey
    if ([WALLET_TYPES.PUBLICKEY, WALLET_TYPES.PRIVATEKEY].includes(this.walletType)) {
      this.keyChainStore
        .getMasterKeyChain()
        .getForPath('0', { isWatched: true });
    }

    this.cacheTx = (opts.cacheTx) ? opts.cacheTx : defaultOptions.cacheTx;
    this.cacheBlockHeaders = (opts.cacheBlockHeaders)
      ? opts.cacheBlockHeaders
      : defaultOptions.cacheBlockHeaders;

    this.plugins = {
      workers: {},
      standard: {},
      watchers: {},
    };

    this.emit(EVENTS.CREATED, { type: EVENTS.CREATED, payload: null });

    /**
     * Stores promise that waits for the transaction FETCH event
     * @type {Promise<void>}
     */
    this.txFetchListener = null;

    this.broadcastRetryAttempts = 0;

    // Increases a limit of max listeners for transactions related events
    // 25 - mempool limit
    this.setMaxListeners(25);
  }

  static getInstantLockTopicName(transactionHash) {
    return `${EVENTS.INSTANT_LOCK}:${transactionHash}`;
  }

  // It's actually Account that mutates wallet.accounts to add itself.
  // We might want to get rid of that as it can be really confusing.
  // It would gives that responsability to createAccount to create
  // (and therefore push to accounts).
  async init(wallet) {
    await _addAccountToWallet(this, wallet);
    await _initializeAccount(this, wallet.plugins);
  }

  async isInitialized() {
    // eslint-disable-next-line consistent-return
    return new Promise(((resolve) => {
      if (this.state.isInitialized) return resolve(true);
      this.on(EVENTS.INITIALIZED, () => resolve(true));
    }));
  }

  async isReady() {
    // eslint-disable-next-line consistent-return
    return new Promise(((resolve) => {
      if (this.state.isReady) return resolve(true);
      this.on(EVENTS.READY, () => resolve(true));
    }));
  }

  /**
   * Imports instant lock to an account and emits message
   * @param {InstantLock} instantLock
   */
  importInstantLock(instantLock) {
    const chainStore = this.storage.getChainStore(this.network);
    chainStore.importInstantLock(instantLock);
    this.emit(Account.getInstantLockTopicName(instantLock.txid), instantLock);
  }

  /**
   * @param {string} transactionHash
   * @param {function} callback
   */
  subscribeToTransactionInstantLock(transactionHash, callback) {
    const eventName = Account.getInstantLockTopicName(transactionHash);

    this.once(eventName, callback);

    return () => {
      this.removeListener(eventName, callback);
    };
  }

  /**
   * @param {string} transactionHash
   * @param {function} callback
   * @returns {function} - cancel subscription
   */
  subscribeToTxMetadata(transactionHash, callback) {
    const eventName = `${EVENTS.TX_METADATA}:${transactionHash}`;

    this.once(eventName, callback);

    return () => {
      this.removeListener(eventName, callback);
    };
  }

  /**
   * Waits for instant lock for a transaction or throws after a timeout
   * @param {string} transactionHash - instant lock to wait for
   * @param {number} timeout - in milliseconds before throwing an error if the lock didn't arrive
   * @return {{promise: Promise<InstantLock>, cancel: Function}}
   */
  waitForInstantLock(transactionHash, timeout = this.waitForInstantLockTimeout) {
    // Return instant lock immediately if already exists
    const chainStore = this.storage.getChainStore(this.network);
    const instantLock = chainStore.getInstantLock(transactionHash);
    if (instantLock != null) {
      return {
        promise: Promise.resolve(instantLock),
        cancel: () => {},
      };
    }

    let rejectTimeout;
    let cancelSubscription;

    function cancel() {
      cancelSubscription();
      clearTimeout(rejectTimeout);
    }

    // Wait for upcoming instant lock

    const promise = Promise.race([
      new Promise((resolve) => {
        cancelSubscription = this.subscribeToTransactionInstantLock(
          transactionHash,
          (instantLockData) => {
            clearTimeout(rejectTimeout);
            resolve(instantLockData);
          },
        );
      }),
      new Promise((resolve, reject) => {
        rejectTimeout = setTimeout(() => {
          cancelSubscription();
          reject(new InstantLockTimeoutError(transactionHash));
        }, timeout);
      }),
    ]);

    return {
      promise,
      cancel,
    };
  }

  /**
   * Waits for metadata of a transaction or throws an error after a timeout
   * @param {string} transactionHash - metadata of tx to wait for
   * @param {number} timeout - in ms before throwing an error if the metadata didn't arrive
   * @return {{promise: Promise<InstantLock>, cancel: Function}}
   */
  waitForTxMetadata(transactionHash, timeout = this.waitForTxMetadataTimeout) {
    // Return tx metadata immediately if already exists
    const chainStore = this.storage.getChainStore(this.network);
    const txWithMetadata = chainStore.getTransaction(transactionHash);

    if (txWithMetadata && txWithMetadata.metadata && txWithMetadata.metadata.height) {
      return {
        promise: Promise.resolve(txWithMetadata.metadata),
        cancel: () => {},
      };
    }

    // Wait for upcoming metadata

    let rejectTimeout;
    let cancelSubscription;

    function cancel() {
      cancelSubscription();
      clearTimeout(rejectTimeout);
    }

    const promise = Promise.race([
      new Promise((resolve) => {
        cancelSubscription = this.subscribeToTxMetadata(transactionHash, (metadata) => {
          clearTimeout(rejectTimeout);
          resolve(metadata);
        });
      }),
      new Promise((resolve, reject) => {
        rejectTimeout = setTimeout(() => {
          cancelSubscription();
          reject(new TxMetadataTimeoutError(transactionHash));
        }, timeout);
      }),
    ]);

    return {
      promise,
      cancel,
    };
  }
}

Account.prototype.broadcastTransaction = require('./methods/broadcastTransaction');
Account.prototype.connect = require('./methods/connect');
Account.prototype.createTransaction = require('./methods/createTransaction');
Account.prototype.decode = require('./methods/decode');
Account.prototype.decrypt = require('./methods/decrypt');
Account.prototype.disconnect = require('./methods/disconnect');
Account.prototype.encode = require('./methods/encode');
Account.prototype.encrypt = require('./methods/encrypt');
Account.prototype.fetchStatus = require('./methods/fetchStatus');
Account.prototype.forceRefreshAccount = require('./methods/forceRefreshAccount');
Account.prototype.generateAddress = require('./methods/generateAddress');
Account.prototype.getAddress = require('./methods/getAddress');
Account.prototype.getAddresses = require('./methods/getAddresses');
Account.prototype.getBlockHeader = require('./methods/getBlockHeader');
Account.prototype.getConfirmedBalance = require('./methods/getConfirmedBalance');
Account.prototype.getPlugin = require('./methods/getPlugin');
Account.prototype.getPrivateKeys = require('./methods/getPrivateKeys');
Account.prototype.getTotalBalance = require('./methods/getTotalBalance');
Account.prototype.getTransaction = require('./methods/getTransaction');
Account.prototype.getTransactionHistory = require('./methods/getTransactionHistory');
Account.prototype.getTransactions = require('./methods/getTransactions');
Account.prototype.getUnconfirmedBalance = require('./methods/getUnconfirmedBalance');
Account.prototype.getUnusedAddress = require('./methods/getUnusedAddress');
Account.prototype.getUnusedIdentityIndex = require('./methods/getUnusedIdentityIndex');
Account.prototype.getUTXOS = require('./methods/getUTXOS');
Account.prototype.getWorker = require('./methods/getWorker');
Account.prototype.hasPlugins = require('./methods/hasPlugins');
Account.prototype.injectPlugin = require('./methods/injectPlugin');
Account.prototype.importTransactions = require('./methods/importTransactions');
Account.prototype.importBlockHeader = require('./methods/importBlockHeader');
Account.prototype.createPathsForTransactions = require('./methods/createPathsForTransactions');
Account.prototype.generateNewPaths = require('./methods/generateNewPaths');
Account.prototype.addPathsToStore = require('./methods/addPathsToStore');
Account.prototype.addDefaultPaths = require('./methods/addDefaultPaths');
Account.prototype.sign = require('./methods/sign');

module.exports = Account;
