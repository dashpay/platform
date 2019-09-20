const { merge } = require('lodash');
const { hasProp } = require('../../../utils');

const mergeHelper = (initial = {}, additional = {}) => merge(initial, additional);
const { REHYDRATE_STATE_FAILED, REHYDRATE_STATE_SUCCESS } = require('../../../EVENTS');

/**
 * Fetch the state from the persistance adapter
 * @return {Promise<void>}
 */
const rehydrateState = async function () {
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
      this.events.emit(REHYDRATE_STATE_SUCCESS);
    } catch (e) {
      this.events.emit(REHYDRATE_STATE_FAILED, e);
      throw e;
    }
  }
};
module.exports = rehydrateState;
