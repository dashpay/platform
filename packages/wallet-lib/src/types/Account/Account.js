import _ from 'lodash';
import EventEmitter from 'events';
import logger from '../../logger/index.js';
import { WALLET_TYPES, BIP44_ADDRESS_GAP } from '../../CONSTANTS.js';
import { is } from '../../utils/index.js';
import EVENTS from '../../EVENTS.js';
import Wallet from '../Wallet/Wallet.js';
import { simpleDescendingAccumulator } from '../../utils/coinSelections/strategies/index.js';
import {
  TxMetadataTimeoutError,
  InstantLockTimeoutError,
} from '../../errors/index.js';

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
import _initializeAccount from './_initializeAccount.js';
import _addAccountToWallet from './_addAccountToWallet.js';
import _loadStrategy from './_loadStrategy.js';

import getNetwork from './_getNetwork.js';
import getBIP44Path from './_getBIP44Path.js';

class Account extends EventEmitter {
  constructor(wallet, opts = defaultOptions) {
    super();
    if (!wallet || wallet.constructor.name !== Wallet.name) throw new Error('Expected wallet to be passed as param');
    if (!_.has(wallet, 'walletId')) throw new Error('Missing walletID to create an account');
    this.walletId = wallet.walletId;
    this.wallet = wallet;
    this.logger = logger.getForWallet(this.walletId);

    this.logger.debug(`Loading up wallet ${this.walletId}`);

    this.identities = wallet.identities;

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

    this.on(
      EVENTS.HEADERS_SYNC_PROGRESS,
      (data) => wallet.emit(EVENTS.HEADERS_SYNC_PROGRESS, data),
    );
    this.on(
      EVENTS.TRANSACTIONS_SYNC_PROGRESS,
      (data) => wallet.emit(EVENTS.TRANSACTIONS_SYNC_PROGRESS, data),
    );
    this.on(
      EVENTS.CONFIRMED_TRANSACTION,
      (data) => wallet.emit(EVENTS.CONFIRMED_TRANSACTION, data),
    );
    this.on(
      EVENTS.BLOCKHEIGHT_CHANGED,
      (data) => wallet.emit(EVENTS.BLOCKHEIGHT_CHANGED, data),
    );

    if (this.debug) {
      this.emit = (...args) => {
        const { type } = args[1];
        const payload = JSON.stringify(args[1].payload);
        this.logger.debug(`${this.walletId}:${this.index} - Emitted event ${type} - ${payload} `);
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

  async init(wallet) {
    if (this.state.isInitialized) {
      return true;
    }
    await _addAccountToWallet(this, wallet);
    await _initializeAccount(this, wallet ? wallet.plugins : this.wallet.plugins);
    return true;
  }

  async isInitialized() {
    // eslint-disable-next-line consistent-return
    return new Promise(((resolve) => {
      if (this.state.isInitialized) {
        resolve(true);
      } else {
        this.on(EVENTS.INITIALIZED, () => {
          resolve(true);
        });
      }
    }));
  }

  async isReady() {
    // eslint-disable-next-line consistent-return
    return new Promise(((resolve) => {
      if (this.state.isReady) {
        resolve(true);
      } else {
        this.on(EVENTS.READY, () => {
          resolve(true);
        });
      }
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
        cancel: () => {
        },
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
        cancel: () => {
        },
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

import _Account_broadcastTransaction from './methods/broadcastTransaction.js';
Account.prototype.broadcastTransaction = _Account_broadcastTransaction;
import _Account_connect from './methods/connect.js';
Account.prototype.connect = _Account_connect;
import _Account_createTransaction from './methods/createTransaction.js';
Account.prototype.createTransaction = _Account_createTransaction;
import _Account_decode from './methods/decode.js';
Account.prototype.decode = _Account_decode;
import _Account_decrypt from './methods/decrypt.js';
Account.prototype.decrypt = _Account_decrypt;
import _Account_disconnect from './methods/disconnect.js';
Account.prototype.disconnect = _Account_disconnect;
import _Account_encode from './methods/encode.js';
Account.prototype.encode = _Account_encode;
import _Account_encrypt from './methods/encrypt.js';
Account.prototype.encrypt = _Account_encrypt;
import _Account_fetchStatus from './methods/fetchStatus.js';
Account.prototype.fetchStatus = _Account_fetchStatus;
import _Account_forceRefreshAccount from './methods/forceRefreshAccount.js';
Account.prototype.forceRefreshAccount = _Account_forceRefreshAccount;
import _Account_generateAddress from './methods/generateAddress.js';
Account.prototype.generateAddress = _Account_generateAddress;
import _Account_getAddress from './methods/getAddress.js';
Account.prototype.getAddress = _Account_getAddress;
import _Account_getAddresses from './methods/getAddresses.js';
Account.prototype.getAddresses = _Account_getAddresses;
import _Account_getBlockHeader from './methods/getBlockHeader.js';
Account.prototype.getBlockHeader = _Account_getBlockHeader;
import _Account_getConfirmedBalance from './methods/getConfirmedBalance.js';
Account.prototype.getConfirmedBalance = _Account_getConfirmedBalance;
import _Account_getPlugin from './methods/getPlugin.js';
Account.prototype.getPlugin = _Account_getPlugin;
import _Account_getPrivateKeys from './methods/getPrivateKeys.js';
Account.prototype.getPrivateKeys = _Account_getPrivateKeys;
import _Account_getTotalBalance from './methods/getTotalBalance.js';
Account.prototype.getTotalBalance = _Account_getTotalBalance;
import _Account_getTransaction from './methods/getTransaction.js';
Account.prototype.getTransaction = _Account_getTransaction;
import _Account_getTransactionHistory from './methods/getTransactionHistory.js';
Account.prototype.getTransactionHistory = _Account_getTransactionHistory;
import _Account_getTransactions from './methods/getTransactions.js';
Account.prototype.getTransactions = _Account_getTransactions;
import _Account_getUnconfirmedBalance from './methods/getUnconfirmedBalance.js';
Account.prototype.getUnconfirmedBalance = _Account_getUnconfirmedBalance;
import _Account_getUnusedAddress from './methods/getUnusedAddress.js';
Account.prototype.getUnusedAddress = _Account_getUnusedAddress;
import _Account_getUnusedIdentityIndex from './methods/getUnusedIdentityIndex.js';
Account.prototype.getUnusedIdentityIndex = _Account_getUnusedIdentityIndex;
import _Account_getUTXOS from './methods/getUTXOS.js';
Account.prototype.getUTXOS = _Account_getUTXOS;
import _Account_getWorker from './methods/getWorker.js';
Account.prototype.getWorker = _Account_getWorker;
import _Account_hasPlugins from './methods/hasPlugins.js';
Account.prototype.hasPlugins = _Account_hasPlugins;
import _Account_injectPlugin from './methods/injectPlugin.js';
Account.prototype.injectPlugin = _Account_injectPlugin;
import _Account_importTransactions from './methods/importTransactions.js';
Account.prototype.importTransactions = _Account_importTransactions;
import _Account_createPathsForTransactions from './methods/createPathsForTransactions.js';
Account.prototype.createPathsForTransactions = _Account_createPathsForTransactions;
import _Account_generateNewPaths from './methods/generateNewPaths.js';
Account.prototype.generateNewPaths = _Account_generateNewPaths;
import _Account_addPathsToStore from './methods/addPathsToStore.js';
Account.prototype.addPathsToStore = _Account_addPathsToStore;
import _Account_addDefaultPaths from './methods/addDefaultPaths.js';
Account.prototype.addDefaultPaths = _Account_addDefaultPaths;
import _Account_sign from './methods/sign.js';
Account.prototype.sign = _Account_sign;

export default Account;