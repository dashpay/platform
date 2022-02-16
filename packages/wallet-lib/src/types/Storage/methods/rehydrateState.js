const { merge } = require('lodash');
const { hasMethod } = require('../../../utils');

const mergeHelper = (initial = {}, additional = {}) => merge(initial, additional);
const { REHYDRATE_STATE_FAILED, REHYDRATE_STATE_SUCCESS } = require('../../../EVENTS');
const WalletStore = require('../../WalletStore/WalletStore');
const ChainStore = require('../../ChainStore/ChainStore');

/**
 * Fetch the state from the persistence adapter
 * @return {Promise<void>}
 */
const rehydrateState = async function rehydrateState() {
  if (this.rehydrate && this.lastRehydrate === null) {
    try {
      if (this.adapter && hasMethod(this.adapter, 'getItem')) {
        const wallets = await this.adapter.getItem('wallets');
        if (wallets) {
          wallets.forEach((walletState) => {
            const walletStore = new WalletStore();
            walletStore.importState(walletState);
            this.wallets.set(walletStore.walletId, walletStore);
          });
        }

        const chains = await this.adapter.getItem('chains');
        if (chains) {
          chains.forEach((chainState) => {
            const chainStore = new ChainStore();
            chainStore.importState(chainState);
            this.chains.set(chainStore.network, chainStore);
          });
        }
        this.application = mergeHelper(this.application, await this.adapter.getItem('application'));
      }

      this.lastRehydrate = +new Date();
      this.emit(REHYDRATE_STATE_SUCCESS, { type: REHYDRATE_STATE_SUCCESS, payload: null });
    } catch (e) {
      this.emit(REHYDRATE_STATE_FAILED, { type: REHYDRATE_STATE_FAILED, payload: e });
      throw e;
    }
  }
};
module.exports = rehydrateState;
