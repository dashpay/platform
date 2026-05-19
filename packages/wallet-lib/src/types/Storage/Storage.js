import EventEmitter from 'events';
import lodash from 'lodash';
const { has } = lodash;
import CONSTANTS from '../../CONSTANTS.js';

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

import _Storage_configure from './methods/configure.js';
Storage.prototype.configure = _Storage_configure;
import _Storage_createChainStore from './methods/createChainStore.js';
Storage.prototype.createChainStore = _Storage_createChainStore;
import _Storage_createWalletStore from './methods/createWalletStore.js';
Storage.prototype.createWalletStore = _Storage_createWalletStore;
import _Storage_getChainStore from './methods/getChainStore.js';
Storage.prototype.getChainStore = _Storage_getChainStore;
import _Storage_getWalletStore from './methods/getWalletStore.js';
Storage.prototype.getWalletStore = _Storage_getWalletStore;
import _Storage_rehydrateState from './methods/rehydrateState.js';
Storage.prototype.rehydrateState = _Storage_rehydrateState;
import _Storage_saveState from './methods/saveState.js';
Storage.prototype.saveState = _Storage_saveState;
import _Storage_startWorker from './methods/startWorker.js';
Storage.prototype.startWorker = _Storage_startWorker;
import _Storage_stopWorker from './methods/stopWorker.js';
Storage.prototype.stopWorker = _Storage_stopWorker;

export default Storage;