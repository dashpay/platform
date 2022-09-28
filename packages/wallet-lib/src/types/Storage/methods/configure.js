const { has } = require('lodash');
const InMem = require('../../../adapters/InMem');
const configureAdapter = require('../_configureAdapter');
const getDefaultAdapter = require('../_getDefaultAdapter');
const { CONFIGURED } = require('../../../EVENTS');
const logger = require('../../../logger');
const CONSTANTS = require('../../../CONSTANTS');

/**
 * To be called after instantialization as it contains all the async logic / test of adapters
 * @param opts
 * @return {Promise<void>}
 */
module.exports = async function configure(opts = {}) {
  this.rehydrate = has(opts, 'rehydrate') ? opts.rehydrate : this.rehydrate;
  this.autosave = has(opts, 'autosave') ? opts.autosave : this.autosave;
  this.adapter = await configureAdapter((opts.adapter) ? opts.adapter : await getDefaultAdapter());

  const storage = await this.adapter.getItem(`wallet_${opts.walletId}`);
  const storageVersion = storage && storage.version;

  if (!(this.adapter instanceof InMem) && storageVersion !== CONSTANTS.STORAGE.version) {
    if (typeof version === 'number') {
      logger.warn('Storage version mismatch, resyncing from start');
    }

    await this.adapter.setItem(`wallet_${opts.walletId}`, null);
  }

  this.createWalletStore(opts.walletId);
  this.createChainStore(opts.network);

  this.currentWalletId = opts.walletId;
  this.currentNetwork = opts.network;

  if (this.rehydrate) {
    await this.rehydrateState();
  }

  if (this.autosave) {
    this.startWorker();
  }

  this.configured = true;
  this.emit(CONFIGURED, { type: CONFIGURED, payload: null });
};
