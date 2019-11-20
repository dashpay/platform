const _ = require('lodash');
const { EventEmitter } = require('events');
const logger = require('../../logger');
const { WALLET_TYPES } = require('../../CONSTANTS');
const { is } = require('../../utils');
const EVENTS = require('../../EVENTS');
const Wallet = require('../Wallet/Wallet.js');
const { simpleTransactionOptimizedAccumulator } = require('../../utils/coinSelections/strategies');

const defaultOptions = {
  network: 'testnet',
  cacheTx: true,
  allowSensitiveOperations: false,
  plugins: [],
  injectDefaultPlugins: true,
  strategy: simpleTransactionOptimizedAccumulator,
};

/* eslint-disable no-underscore-dangle */
const _initializeAccount = require('./_initializeAccount');
const _addAccountToWallet = require('./_addAccountToWallet');
const _loadStrategy = require('./_loadStrategy');

const getNetwork = require('./_getNetwork');
const getBIP44Path = require('./_getBIP44Path');

class Account {
  constructor(wallet, opts = defaultOptions) {
    if (!wallet || wallet.constructor.name !== Wallet.name) throw new Error('Expected wallet to be passed as param');
    if (!_.has(wallet, 'walletId')) throw new Error('Missing walletID to create an account');
    this.walletId = wallet.walletId;

    this.events = new EventEmitter();
    this.isReady = false;
    this.isInitialized = false;
    this.isDisconnecting = false;
    this.injectDefaultPlugins = _.has(opts, 'injectDefaultPlugins') ? opts.injectDefaultPlugins : defaultOptions.injectDefaultPlugins;
    this.allowSensitiveOperations = _.has(opts, 'allowSensitiveOperations') ? opts.allowSensitiveOperations : defaultOptions.allowSensitiveOperations;

    this.walletType = wallet.walletType;
    this.offlineMode = wallet.offlineMode;

    const accountIndex = _.has(opts, 'accountIndex') ? opts.accountIndex : wallet.accounts.length;
    this.accountIndex = accountIndex;
    this.strategy = _loadStrategy(_.has(opts, 'strategy') ? opts.strategy : defaultOptions.strategy);
    this.network = getNetwork(wallet.network).toString();

    this.BIP44PATH = getBIP44Path(this.network, accountIndex);

    this.transactions = {};

    this.label = (opts && opts.label && is.string(opts.label)) ? opts.label : null;

    // If transport is null or invalid, we won't try to fetch anything
    this.transport = wallet.transport;

    this.store = wallet.storage.store;
    this.storage = wallet.storage;

    if (this.walletType === WALLET_TYPES.HDWALLET) {
      this.storage.importAccounts({
        label: this.label,
        path: this.BIP44PATH,
        network: this.network,
      }, this.walletId);
    }
    if (this.walletType === WALLET_TYPES.HDEXTPUBLIC) {
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

    this.plugins = {
      workers: {},
      DPAs: {},
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
          logger.error(e);
          this.disconnect();
          throw e;
        }
      }
    }
    this.events.emit(EVENTS.CREATED);
    _addAccountToWallet(this, wallet);
    _initializeAccount(this, wallet.plugins);
  }
}

Account.prototype.broadcastTransaction = require('./methods/broadcastTransaction');
Account.prototype.connect = require('./methods/connect');
Account.prototype.createTransaction = require('./methods/createTransaction');
Account.prototype.disconnect = require('./methods/disconnect');
Account.prototype.fetchAddressInfo = require('./methods/fetchAddressInfo');
Account.prototype.fetchStatus = require('./methods/fetchStatus');

Account.prototype.fetchTransactionInfo = require('./methods/fetchTransactionInfo');

Account.prototype.forceRefreshAccount = require('./methods/forceRefreshAccount');

Account.prototype.encrypt = require('./methods/encrypt');

Account.prototype.encode = require('./methods/encode');

Account.prototype.generateAddress = require('./methods/generateAddress');

Account.prototype.getAddress = require('./methods/getAddress');

Account.prototype.getAddresses = require('./methods/getAddresses');

Account.prototype.getConfirmedBalance = require('./methods/getConfirmedBalance');

Account.prototype.getUnconfirmedBalance = require('./methods/getUnconfirmedBalance');

Account.prototype.getTotalBalance = require('./methods/getTotalBalance');

Account.prototype.getDPA = require('./methods/getDPA');

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

Account.prototype.updateNetwork = require('./methods/updateNetwork');

Account.prototype.hasPlugins = require('./methods/hasPlugins');

Account.prototype.getIdentityPrivateKey = require('./methods/getIdentityPrivateKey');

module.exports = Account;
