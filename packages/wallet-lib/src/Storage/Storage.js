const { EventEmitter } = require('events');
const { cloneDeep, has } = require('lodash');

const CONSTANTS = require('../CONSTANTS');
const addNewTxToAddress = require('./addNewTxToAddress');
const addUTXOToAddress = require('./addUTXOToAddress');
const announce = require('./announce');
const calculateDuffBalance = require('./calculateDuffBalance');
const clearAll = require('./clearAll');
const configure = require('./configure');
const createChain = require('./createChain');
const createWallet = require('./createWallet');
const getStore = require('./getStore');
const getTransaction = require('./getTransaction');
const importAccounts = require('./importAccounts');
const importAddress = require('./importAddress');
const importAddresses = require('./importAddresses');
const importSingleAddress = require('./importSingleAddress');
const importTransaction = require('./importTransaction');
const importTransactions = require('./importTransactions');

const rehydrateState = require('./rehydrateState');
const saveState = require('./saveState');
const searchAddress = require('./searchAddress');
const searchAddressesWithTx = require('./searchAddressesWithTx');
const searchTransaction = require('./searchTransaction');
const searchWallet = require('./searchWallet');
const updateAddress = require('./updateAddress');
const updateTransaction = require('./updateTransaction');
const startWorker = require('./startWorker');
const stopWorker = require('./stopWorker');

const initialStore = {
  wallets: {},
  transactions: {},
  chains: {},
};
// eslint-disable-next-line no-underscore-dangle
const _defaultOpts = {
  rehydrate: true,
  autosave: true,
  autosaveIntervalTime: CONSTANTS.STORAGE.autosaveIntervalTime,
  network: 'testnet',
};
/**
 * Handle all the storage logic, it's a wrapper around the adapters
 * So all the needed methods should be provided by the Storage class and the access to the adapter
 * should be limited.
 * */
class Storage {
  constructor(opts = JSON.parse(JSON.stringify(_defaultOpts))) {
    const defaultOpts = JSON.parse(JSON.stringify(_defaultOpts));

    this.events = new EventEmitter();
    this.store = cloneDeep(initialStore);
    this.rehydrate = has(opts, 'rehydrate') ? opts.rehydrate : defaultOpts.rehydrate;
    this.autosave = has(opts, 'autosave') ? opts.autosave : defaultOpts.autosave;
    this.autosaveIntervalTime = has(opts, 'autosaveIntervalTime')
      ? opts.autosaveIntervalTime
      : defaultOpts.autosaveIntervalTime;

    this.lastRehydrate = null;
    this.lastSave = null;
    this.lastModified = null;
    this.network = has(opts, 'network') ? opts.network.toString() : defaultOpts.network;
    // // Map an address to it's walletid/path/type schema (used by searchAddress for speedup)
    this.mappedAddress = {};
  }

  attachEvents(events) {
    this.events = events;
  }
}
Storage.prototype.addNewTxToAddress = addNewTxToAddress;
Storage.prototype.addUTXOToAddress = addUTXOToAddress;
Storage.prototype.announce = announce;
Storage.prototype.calculateDuffBalance = calculateDuffBalance;
Storage.prototype.clearAll = clearAll;
Storage.prototype.configure = configure;
Storage.prototype.createChain = createChain;
Storage.prototype.createWallet = createWallet;
Storage.prototype.getStore = getStore;
Storage.prototype.getTransaction = getTransaction;
Storage.prototype.importAccounts = importAccounts;
Storage.prototype.importAddress = importAddress;
Storage.prototype.importAddresses = importAddresses;
Storage.prototype.importSingleAddress = importSingleAddress;
Storage.prototype.importTransaction = importTransaction;
Storage.prototype.importTransactions = importTransactions;
Storage.prototype.rehydrateState = rehydrateState;
Storage.prototype.saveState = saveState;
Storage.prototype.searchAddress = searchAddress;
Storage.prototype.searchAddressesWithTx = searchAddressesWithTx;
Storage.prototype.searchTransaction = searchTransaction;
Storage.prototype.searchWallet = searchWallet;
Storage.prototype.updateAddress = updateAddress;
Storage.prototype.updateTransaction = updateTransaction;
Storage.prototype.startWorker = startWorker;
Storage.prototype.stopWorker = stopWorker;

module.exports = Storage;
