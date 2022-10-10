const EventEmitter = require('events');
const { has } = require('lodash');
const CONSTANTS = require('../../CONSTANTS');

const defaultOpts = {
  rehydrate: true,
  autosave: true,
  purgeOnError: true,
  autosaveIntervalTime: CONSTANTS.STORAGE.autosaveIntervalTime,
  network: 'testnet',
};

/**
* Handle all the storage logic, it's a wrapper around the adapters
* So all the needed methods should be provided by the Storage class and the access to the adapter
* should be limited.
* */
class Storage extends EventEmitter {
  constructor(opts = {}) {
    super();
    this.currentWalletId = '';
    this.currentNetwork = '';
    this.wallets = new Map();
    this.chains = new Map();
    this.application = {};

    this.rehydrate = has(opts, 'rehydrate') ? opts.rehydrate : defaultOpts.rehydrate;
    this.autosave = has(opts, 'autosave') ? opts.autosave : defaultOpts.autosave;
    this.purgeOnError = has(opts, 'purgeOnError') ? opts.purgeOnError : defaultOpts.purgeOnError;
    this.autosaveIntervalTime = has(opts, 'autosaveIntervalTime')
      ? opts.autosaveIntervalTime
      : defaultOpts.autosaveIntervalTime;

    this.lastRehydrate = null;
    this.lastSave = null;
    this.lastModified = null;
    this.configured = false;
  }

  reset() {
    this.wallets.forEach((wallet) => wallet.reset());
    this.chains.forEach((chain) => chain.reset());
    this.lastRehydrate = null;
  }

  scheduleStateSave() {
    this.lastModified = Date.now();
  }

  getDefaultChainStore() {
    return this.getChainStore(this.currentNetwork);
  }

  getDefaultWalletStore() {
    return this.getWalletStore(this.currentWalletId);
  }
}

Storage.prototype.configure = require('./methods/configure');
Storage.prototype.createChainStore = require('./methods/createChainStore');
Storage.prototype.createWalletStore = require('./methods/createWalletStore');
Storage.prototype.getChainStore = require('./methods/getChainStore');
Storage.prototype.getWalletStore = require('./methods/getWalletStore');
Storage.prototype.rehydrateState = require('./methods/rehydrateState');
Storage.prototype.saveState = require('./methods/saveState');
Storage.prototype.startWorker = require('./methods/startWorker');
Storage.prototype.stopWorker = require('./methods/stopWorker');

module.exports = Storage;
