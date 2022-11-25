const { SAVE_STATE_SUCCESS, SAVE_STATE_FAILED } = require('../../../EVENTS');
const CONSTANTS = require('../../../CONSTANTS');

/**
 * Force persistence of the state to the adapter
 * @return {Promise<boolean>}
 */
const saveState = async function saveState() {
  if (this.autosave && this.adapter && this.adapter.setItem) {
    this.lastSave = Date.now();

    const self = this;
    try {
      const serializedChains = [...self.chains].reduce((acc, [chainId, chainStore]) => {
        acc[chainId] = chainStore.exportState();
        return acc;
      }, {});

      const { skipSynchronizationBeforeHeight } = this.application.syncOptions || {};
      const skipSyncBeforeHeight = parseInt(skipSynchronizationBeforeHeight, 10);

      const walletId = this.currentWalletId;
      const storage = {
        version: CONSTANTS.STORAGE.version,
        chains: {},
        // Memorize skipSynchronizationBeforeHeight option in order to wipe the storage
        // and re-sync from block 1 in case the option is removed or value changed on next launch
        unsafeOptions: {
          skipSynchronizationBeforeHeight:
            !Number.isNaN(skipSyncBeforeHeight) ? skipSyncBeforeHeight : 0,
        },
      };

      Object.keys(serializedChains).forEach((chainNetwork) => {
        storage.chains[chainNetwork] = serializedChains[chainNetwork];
      });

      // eslint-disable-next-line
      await this.adapter.setItem(`wallet_${walletId}`, storage);

      this.emit(SAVE_STATE_SUCCESS, { type: SAVE_STATE_SUCCESS, payload: this.lastSave });
      this.logger.debug(`State saved. Estimated time: ${(Date.now() - this.lastSave) / 1000}s`);
      return true;
    } catch (err) {
      this.logger.error(`Storage Save state error: ${err.message}`);
      this.emit(SAVE_STATE_FAILED, { type: SAVE_STATE_FAILED, payload: err });
      throw err;
    }
  }
  return false;
};
module.exports = saveState;
