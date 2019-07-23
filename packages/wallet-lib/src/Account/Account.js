const _ = require('lodash');
const { EventEmitter } = require('events');

const { WALLET_TYPES } = require('../CONSTANTS');
const { is } = require('../utils/index');
const EVENTS = require('../EVENTS');
const Wallet = require('../Wallet/Wallet.js');
const { simpleTransactionOptimizedAccumulator } = require('../utils/coinSelections/strategies');

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

const broadcastTransaction = require('./broadcastTransaction');
const connect = require('./connect');
const createTransaction = require('./createTransaction');
const disconnect = require('./disconnect');
const fetchAddressInfo = require('./fetchAddressInfo');
const fetchStatus = require('./fetchStatus');
const fetchTransactionInfo = require('./fetchTransactionInfo');
const forceRefreshAccount = require('./forceRefreshAccount');
const generateAddress = require('./generateAddress');
const getAddress = require('./getAddress');
const getAddresses = require('./getAddresses');
const getConfirmedBalance = require('./getConfirmedBalance');
const getUnconfirmedBalance = require('./getUnconfirmedBalance');
const getTotalBalance = require('./getTotalBalance');
const getBIP44Path = require('./getBIP44Path');
const getDPA = require('./getDPA');
const getNetwork = require('./getNetwork');
const getPlugin = require('./getPlugin');
const getWorker = require('./getWorker');
const getPrivateKeys = require('./getPrivateKeys');
const getTransaction = require('./getTransaction');
const getTransactionHistory = require('./getTransactionHistory');
const getTransactions = require('./getTransactions');
const getUnusedAddress = require('./getUnusedAddress');
const getUTXOS = require('./getUTXOS');
const injectPlugin = require('./injectPlugin');
const sign = require('./sign');
const updateNetwork = require('./updateNetwork');
const hasPlugins = require('./hasPlugins');

class Account {
  constructor(wallet, opts = defaultOptions) {
    if (!wallet || wallet.constructor.name !== Wallet.name) throw new Error('Expected wallet to be passed as param');
    if (!_.has(wallet, 'walletId')) throw new Error('Missing walletID to create an account');
    this.walletId = wallet.walletId;

    this.events = new EventEmitter();
    this.isReady = false;
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
          console.log(e);
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

Account.prototype.broadcastTransaction = broadcastTransaction;
Account.prototype.connect = connect;
Account.prototype.createTransaction = createTransaction;
Account.prototype.disconnect = disconnect;
Account.prototype.fetchAddressInfo = fetchAddressInfo;
Account.prototype.fetchStatus = fetchStatus;
Account.prototype.fetchTransactionInfo = fetchTransactionInfo;
Account.prototype.forceRefreshAccount = forceRefreshAccount;
Account.prototype.generateAddress = generateAddress;
Account.prototype.getAddress = getAddress;
Account.prototype.getAddresses = getAddresses;
Account.prototype.getConfirmedBalance = getConfirmedBalance;
Account.prototype.getUnconfirmedBalance = getUnconfirmedBalance;
Account.prototype.getTotalBalance = getTotalBalance;
Account.prototype.getBIP44Path = getBIP44Path;
Account.prototype.getDPA = getDPA;
Account.prototype.getPlugin = getPlugin;
Account.prototype.getWorker = getWorker;
Account.prototype.getPrivateKeys = getPrivateKeys;
Account.prototype.getTransaction = getTransaction;
Account.prototype.getTransactionHistory = getTransactionHistory;
Account.prototype.getTransactions = getTransactions;
Account.prototype.getUnusedAddress = getUnusedAddress;
Account.prototype.getUTXOS = getUTXOS;
Account.prototype.injectPlugin = injectPlugin;
Account.prototype.sign = sign;
Account.prototype.updateNetwork = updateNetwork;
Account.prototype.hasPlugins = hasPlugins;

module.exports = Account;
