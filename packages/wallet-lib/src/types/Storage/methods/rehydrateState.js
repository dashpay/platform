const { merge } = require('lodash');
const { hasProp } = require('../../../utils');

const mergeHelper = (initial = {}, additional = {}) => merge(initial, additional);
const { REHYDRATE_STATE_FAILED, REHYDRATE_STATE_SUCCESS } = require('../../../EVENTS');

/**
 * Fetch the state from the persistence adapter
 * @return {Promise<void>}
 */
const rehydrateState = async function rehydrateState() {
  if (this.rehydrate && this.lastRehydrate === null) {
    try {
      const transactions = (this.adapter && hasProp(this.adapter, 'getItem'))
        ? (await this.adapter.getItem('transactions') || this.store.transactions)
        : this.store.transactions;
      const wallets = (this.adapter && hasProp(this.adapter, 'getItem'))
        ? (await this.adapter.getItem('wallets') || this.store.wallets)
        : this.store.wallets;
      const chains = (this.adapter && hasProp(this.adapter, 'getItem'))
        ? (await this.adapter.getItem('chains') || this.store.chains)
        : this.store.chains;

      this.store.transactions = mergeHelper(this.store.transactions, transactions);
      this.store.wallets = mergeHelper(this.store.wallets, wallets);
      this.store.chains = mergeHelper(this.store.chains, chains);
      this.lastRehydrate = +new Date();
      this.emit(REHYDRATE_STATE_SUCCESS, { type: REHYDRATE_STATE_SUCCESS, payload: null });
    } catch (e) {
      this.emit(REHYDRATE_STATE_FAILED, { type: REHYDRATE_STATE_FAILED, payload: e });
      throw e;
    }
  }
};
module.exports = rehydrateState;
