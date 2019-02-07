const { SAVE_STATE_SUCCESS, SAVE_STATE_FAILED } = require('../EVENTS');

/**
 * Force persistance of the state to the adapter
 * @return {Promise<boolean>}
 */
const saveState = async function () {
  if (this.autosave && this.adapter && this.adapter.setItem) {
    const self = this;
    try {
      await this.adapter.setItem('transactions', { ...self.store.transactions });
      await this.adapter.setItem('wallets', { ...self.store.wallets });
      await this.adapter.setItem('chains', { ...self.store.chains });
      this.lastSave = +new Date();
      this.events.emit(SAVE_STATE_SUCCESS);
      return true;
    } catch (e) {
      switch (e.message) {
        default:
          console.log('Storage saveState err', e);
      }

      this.events.emit(SAVE_STATE_FAILED, e);

      throw e;
    }
  }
  return false;
};
module.exports = saveState;
