const _ = require('lodash');
const { EventEmitter2: EventEmitter } = require('eventemitter2');
const logger = require('../../logger');
const { WALLET_TYPES } = require('../../CONSTANTS');
const { is } = require('../../utils');
const EVENTS = require('../../EVENTS');
const Wallet = require('../Wallet/Wallet.js');
const { simpleTransactionOptimizedAccumulator } = require('../../utils/coinSelections/strategies');

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
  strategy: simpleTransactionOptimizedAccumulator,
};

/* eslint-disable no-underscore-dangle */
const _initializeAccount = require('./_initializeAccount');
const _addAccountToWallet = require('./_addAccountToWallet');
const _loadStrategy = require('./_loadStrategy');

const getNetwork = require('./_getNetwork');
const getBIP44Path = require('./_getBIP44Path');

class Account extends EventEmitter {
  constructor(wallet, opts = defaultOptions) {
    super({ wildcard: true });
    if (!wallet || wallet.constructor.name !== Wallet.name) throw new Error('Expected wallet to be passed as param');
    if (!_.has(wallet, 'walletId')) throw new Error('Missing walletID to create an account');
    this.walletId = wallet.walletId;

    this.state = {
      isInitialized: false,
      isReady: false,
      isDisconnecting: false,
    };
    this.injectDefaultPlugins = _.has(opts, 'injectDefaultPlugins') ? opts.injectDefaultPlugins : defaultOptions.injectDefaultPlugins;
    this.allowSensitiveOperations = _.has(opts, 'allowSensitiveOperations') ? opts.allowSensitiveOperations : defaultOptions.allowSensitiveOperations;
    this.debug = _.has(opts, 'debug') ? opts.debug : defaultOptions.debug;
    if (this.debug) process.env.LOG_LEVEL = 'debug';

    this.walletType = wallet.walletType;
    this.offlineMode = wallet.offlineMode;


    this.index = _.has(opts, 'index') ? opts.index : getNextUnusedAccountIndexForWallet(wallet);
    this.strategy = _loadStrategy(_.has(opts, 'strategy') ? opts.strategy : defaultOptions.strategy);
    this.network = getNetwork(wallet.network).toString();

    this.BIP44PATH = getBIP44Path(this.network, this.index);

    this.transactions = {};

    this.label = (opts && opts.label && is.string(opts.label)) ? opts.label : null;

    // If transporter is null or invalid, we won't try to fetch anything
    this.transporter = wallet.transporter;

    this.store = wallet.storage.store;
    this.storage = wallet.storage;

    // Forward all storage event
    this.storage.on('**', (ev) => {
      this.emit(ev.type, ev);
    });
    if (this.debug) {
      this.emit = (...args) => {
        const { type } = args[1];
        const payload = JSON.stringify(args[1].payload);
        logger.debug(`${this.walletId}:${this.index} - Emitted event ${type} - ${payload} `);
        super.emit(...args);
      };
    }
    if (this.walletType === WALLET_TYPES.HDWALLET) {
      this.storage.importAccounts({
        label: this.label,
        path: this.BIP44PATH,
        network: this.network,
      }, this.walletId);
    }
    if (this.walletType === WALLET_TYPES.HDPUBLIC) {
      this.storage.importSingleAddress({
        label: this.label,
        path: '/0',
        network: this.network,
      }, this.walletId);
    }
    if (this.walletType === WALLET_TYPES.SINGLE_ADDRESS) {
      this.storage.importSingleAddress({
        label: this.label,
        path: '0',
        network: this.network,
      }, this.walletId);
    }

    this.keyChain = wallet.keyChain;

    this.cacheTx = (opts.cacheTx) ? opts.cacheTx : defaultOptions.cacheTx;
    this.cacheBlockHeaders = (opts.cacheBlockHeaders)
      ? opts.cacheBlockHeaders
      : defaultOptions.cacheBlockHeaders;

    this.plugins = {
      workers: {},
      standard: {},
      watchers: {},
    };

    // Handle import of cache
    if (opts.cache) {
      if (opts.cache.addresses) {
        try {
          this.storage.importAddresses(opts.cache.addresses, this.walletId);
        } catch (e) {
          this.disconnect();
          throw e;
        }
      }
      if (opts.cache.transactions) {
        try {
          this.storage.importTransactions(opts.cache.transactions);
        } catch (e) {
          this.disconnect();
          throw e;
        }
      }
    }
    this.emit(EVENTS.CREATED, { type: EVENTS.CREATED, payload: null });
    // It's actually Account that mutates wallet.accounts to add itself.
    // We might want to get rid of that as it can be really confusing.
    // It would gives that responsability to createAccount to create
    // (and therefore push to accounts).
    _addAccountToWallet(this, wallet);
    _initializeAccount(this, wallet.plugins);
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
}

Account.prototype.broadcastTransaction = require('./methods/broadcastTransaction');
Account.prototype.connect = require('./methods/connect');
Account.prototype.createTransaction = require('./methods/createTransaction');
Account.prototype.decode = require('./methods/decode');
Account.prototype.decrypt = require('./methods/decrypt');
Account.prototype.disconnect = require('./methods/disconnect');
Account.prototype.fetchAddressInfo = require('./methods/fetchAddressInfo');
Account.prototype.fetchStatus = require('./methods/fetchStatus');

Account.prototype.forceRefreshAccount = require('./methods/forceRefreshAccount');

Account.prototype.encrypt = require('./methods/encrypt');

Account.prototype.encode = require('./methods/encode');

Account.prototype.generateAddress = require('./methods/generateAddress');

Account.prototype.getAddress = require('./methods/getAddress');

Account.prototype.getAddresses = require('./methods/getAddresses');
Account.prototype.getBlockHeader = require('./methods/getBlockHeader');
Account.prototype.getConfirmedBalance = require('./methods/getConfirmedBalance');
Account.prototype.getUnconfirmedBalance = require('./methods/getUnconfirmedBalance');

Account.prototype.getTotalBalance = require('./methods/getTotalBalance');

Account.prototype.getPlugin = require('./methods/getPlugin');

Account.prototype.getWorker = require('./methods/getWorker');

Account.prototype.getPrivateKeys = require('./methods/getPrivateKeys');

Account.prototype.getTransaction = require('./methods/getTransaction');

Account.prototype.getTransactionHistory = require('./methods/getTransactionHistory');

Account.prototype.getTransactions = require('./methods/getTransactions');

Account.prototype.getUnusedAddress = require('./methods/getUnusedAddress');

Account.prototype.getUTXOS = require('./methods/getUTXOS');

Account.prototype.injectPlugin = require('./methods/injectPlugin');

Account.prototype.sign = require('./methods/sign');

Account.prototype.hasPlugins = require('./methods/hasPlugins');

Account.prototype.getIdentityHDKey = require('./methods/getIdentityHDKey');

module.exports = Account;
