const { SAVE_STATE_SUCCESS, SAVE_STATE_FAILED } = require('../../../EVENTS');

/**
 * Force persistence of the state to the adapter
 * @return {Promise<boolean>}
 */
const saveState = async function saveState() {
  if (this.autosave && this.adapter && this.adapter.setItem) {
    const self = this;
    try {
      const serializedWallets = [...self.wallets].reduce((acc, [walletId, walletStore]) => {
        acc[walletId] = walletStore.exportState();
        return acc;
      }, {});

      const serializedChains = [...self.chains].reduce((acc, [chainId, chainStore]) => {
        acc[chainId] = chainStore.exportState();
        return acc;
      }, {});

      await this.adapter.setItem('wallets', serializedWallets);
      await this.adapter.setItem('chains', serializedChains);

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
