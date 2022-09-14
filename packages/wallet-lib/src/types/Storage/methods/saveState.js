const { SAVE_STATE_SUCCESS, SAVE_STATE_FAILED } = require('../../../EVENTS');
const CONSTANTS = require('../../../CONSTANTS');

/**
 * Force persistence of the state to the adapter
 * @return {Promise<boolean>}
 */
const saveState = async function saveState() {
  if (this.autosave && this.adapter && this.adapter.setItem) {
    this.lastSave = +new Date();

    const self = this;
    try {
      const serializedChains = [...self.chains].reduce((acc, [chainId, chainStore]) => {
        acc[chainId] = chainStore.exportState();
        return acc;
      }, {});

      const walletId = this.currentWalletId;
      const storage = { version: CONSTANTS.STORAGE.version, chains: {} };

      Object.keys(serializedChains).forEach((chainNetwork) => {
        storage.chains[chainNetwork] = serializedChains[chainNetwork];
      });

      // eslint-disable-next-line
      await this.adapter.setItem(`wallet_${walletId}`, storage);

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
