const { EventEmitter } = require('events');
const { cloneDeep, has } = require('lodash');

const CONSTANTS = require('../CONSTANTS');
const addNewTxToAddress = require('./addNewTxToAddress');
const addUTXOToAddress = require('./addUTXOToAddress');
const announce = require('./announce');
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
const searchAddressWithTx = require('./searchAddressWithTx');
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
const defaultOpts = {
  rehydrate: true,
  autosave: true,
  autosaveIntervalTime: CONSTANTS.STORAGE.autosaveIntervalTime,
};
/**
 * Handle all the storage logic, it's a wrapper around the adapters
 * So all the needed methods should be provided by the Storage class and the access to the adapter
 * should be limited.
 * */
class Storage {
  constructor(opts = defaultOpts) {
    Object.assign(Storage.prototype, {
      addNewTxToAddress,
      addUTXOToAddress,
      announce,
      clearAll,
      configure,
      createChain,
      createWallet,
      getStore,
      getTransaction,
      importAccounts,
      importAddress,
      importAddresses,
      importSingleAddress,
      importTransaction,
      importTransactions,
      rehydrateState,
      saveState,
      searchAddress,
      searchAddressWithTx,
      searchTransaction,
      searchWallet,
      updateAddress,
      updateTransaction,
      startWorker,
      stopWorker,
    });

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

    // // Map an address to it's walletid/path/type schema (used by searchAddress for speedup)
    this.mappedAddress = {};
  }

  attachEvents(events) {
    this.events = events;
  }
}
module.exports = Storage;
