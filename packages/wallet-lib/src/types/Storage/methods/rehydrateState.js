const { hasMethod } = require('../../../utils');

const { REHYDRATE_STATE_FAILED, REHYDRATE_STATE_SUCCESS } = require('../../../EVENTS');

const logger = require('../../../logger');

/**
 * Fetch the state from the persistence adapter
 * @return {Promise<void>}
 */
const rehydrateState = async function rehydrateState() {
  if (this.rehydrate && this.lastRehydrate === null) {
    try {
      if (this.adapter && hasMethod(this.adapter, 'getItem')) {
        const walletId = this.currentWalletId;
        const storage = await this.adapter.getItem(`wallet_${walletId}`);

        if (storage) {
          try {
            const { chains } = storage;

            Object.keys(chains).forEach((chainNetwork) => {
              const { chain, wallet } = storage.chains[chainNetwork];

              const chainStore = this.getChainStore(chainNetwork);

              if (chainStore) {
                chainStore.importState(chain);
              }

              const walletStore = this.getWalletStore(walletId);

              if (walletStore) {
                walletStore.importState(wallet);
              }
            });
          } catch (e) {
            logger.error('Error importing persistent storage, resyncing from start', e);

            this.adapter.setItem(`wallet_${walletId}`, null);
          }
        }
      }

      this.lastRehydrate = +new Date();
      this.emit(REHYDRATE_STATE_SUCCESS, { type: REHYDRATE_STATE_SUCCESS, payload: null });
    } catch (e) {
      logger.error('Error rehydrating storage state', e);
      this.emit(REHYDRATE_STATE_FAILED, { type: REHYDRATE_STATE_FAILED, payload: e });
      throw e;
    }
  }
};
module.exports = rehydrateState;
