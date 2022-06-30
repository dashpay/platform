const { SAVE_STATE_SUCCESS, SAVE_STATE_FAILED } = require('../../../EVENTS');
const CONSTANTS = require('../../../CONSTANTS');

/**
 * Force persistence of the state to the adapter
 * @return {Promise<boolean>}
 */
const saveState = async function saveState() {
  if (this.autosave && this.adapter && this.adapter.setItem) {
    const self = this;
    try {
      const currentChainHeight = this.getChainStore(this.currentNetwork).state.blockHeight;

      const serializedWallets = [...self.wallets].reduce((acc, [walletId, walletStore]) => {
        let walletStoreState;
        if (walletId === this.currentWalletId) {
          // For current wallet we need to take into account the current chain height
          walletStoreState = walletStore.exportState(currentChainHeight);
        } else {
          // Others stay unaffected
          walletStoreState = walletStore.exportState();
        }

        acc[walletId] = walletStoreState;
        return acc;
      }, {});

      const serializedChains = [...self.chains].reduce((acc, [chainId, chainStore]) => {
        acc[chainId] = chainStore.exportState();
        return acc;
      }, {});

      const walletIds = Object.keys(serializedWallets);
      for (let i = 0; i < walletIds.length; i += 1) {
        const walletId = walletIds[i];
        const storage = { version: CONSTANTS.STORAGE.version, chains: {} };
        const wallet = serializedWallets[walletId];

        Object.keys(serializedChains).forEach((chainNetwork) => {
          const chain = serializedChains[chainNetwork];

          storage.chains[chainNetwork] = { chain, wallet };
        });

        // eslint-disable-next-line
        await this.adapter.setItem(`wallet_${walletId}`, storage);
      }
      // Object.keys(serializedWallets).forEach((walletId) => {
      //
      // });
      // console.log('Saved for', (Date.now() - now) / 1000);
      this.lastSave = +new Date();
      this.emit(SAVE_STATE_SUCCESS, { type: SAVE_STATE_SUCCESS, payload: this.lastSave });
      return true;
    } catch (err) {
      this.emit(SAVE_STATE_FAILED, { type: SAVE_STATE_FAILED, payload: err });
      throw err;
    }
  }
  return false;
};
module.exports = saveState;
