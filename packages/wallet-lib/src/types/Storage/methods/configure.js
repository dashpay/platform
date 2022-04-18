const { has } = require('lodash');
const configureAdapter = require('../_configureAdapter');
const getDefaultAdapter = require('../_getDefaultAdapter');
const { CONFIGURED } = require('../../../EVENTS');
const logger = require('../../../logger');

const CURRENT_VERSION = 1;

/**
 * To be called after instantialization as it contains all the async logic / test of adapters
 * @param opts
 * @return {Promise<void>}
 */
module.exports = async function configure(opts = {}) {
  this.rehydrate = has(opts, 'rehydrate') ? opts.rehydrate : this.rehydrate;
  this.autosave = has(opts, 'autosave') ? opts.autosave : this.autosave;
  this.adapter = await configureAdapter((opts.adapter) ? opts.adapter : await getDefaultAdapter());

  const version = await this.adapter.getItem('version');

  if (!version) {
    await this.adapter.setItem('version', CURRENT_VERSION);
  } else if (version !== CURRENT_VERSION) {
    logger.warn('Storage version mismatch, resyncing from start');
    await this.adapter.setItem('wallets', null);
    await this.adapter.setItem('chains', null);

    await this.adapter.setItem('version', CURRENT_VERSION);
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
