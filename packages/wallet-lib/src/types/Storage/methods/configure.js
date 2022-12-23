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

  if (storage && !(this.adapter instanceof InMem)) {
    if (storageVersion !== CONSTANTS.STORAGE.version) {
      if (typeof storageVersion === 'number') {
        logger.warn('Storage version mismatch, re-syncing from start');
      }

      await this.adapter.setItem(`wallet_${opts.walletId}`, null);
    }

    const { skipSynchronizationBeforeHeight } = this.application.syncOptions || {};
    const skipSync = parseInt(skipSynchronizationBeforeHeight, 10);
    const skipSyncPrev = storage.unsafeOptions
      && storage.unsafeOptions.skipSynchronizationBeforeHeight;

    if (skipSyncPrev && !skipSync) {
      logger.warn('\'skipSynchronizationBeforeHeight\' option has been unset since the last use, re-syncing from start');
      await this.adapter.setItem(`wallet_${opts.walletId}`, null);
    } else if (!skipSyncPrev && skipSync) {
      logger.warn(`'skipSynchronizationBeforeHeight' option has been set, syncing from ${skipSync}`);
      await this.adapter.setItem(`wallet_${opts.walletId}`, null);
    } else if (
      skipSyncPrev && skipSync
      && skipSyncPrev !== skipSync
    ) {
      logger.warn(`'skipSynchronizationBeforeHeight' option has been changed from ${skipSyncPrev} to ${skipSync}, re-syncing.`);
      await this.adapter.setItem(`wallet_${opts.walletId}`, null);
    }
  }

  this.createWalletStore(opts.walletId);
  this.createChainStore(opts.network);

  this.currentWalletId = opts.walletId;
  this.currentNetwork = opts.network;
  this.logger = logger.getForWallet(this.currentWalletId);

  if (this.rehydrate) {
    await this.rehydrateState();
  }

  if (this.autosave) {
    this.startWorker();
  }

  this.configured = true;
  this.emit(CONFIGURED, { type: CONFIGURED, payload: null });
};
