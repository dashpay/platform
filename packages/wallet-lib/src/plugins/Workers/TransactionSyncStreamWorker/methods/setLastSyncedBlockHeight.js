/**
 * Set last synced block height
 *
 * @param  {number} blockHeight
 * @param  {boolean} [updateWalletState=false]
 * @return {number}
 */
module.exports = function setLastSyncedBlockHeight(blockHeight, updateWalletState = false) {
  if (this.lastSyncedBlockHeight >= blockHeight) {
    return this.lastSyncedBlockHeight;
  }

  this.lastSyncedBlockHeight = blockHeight;

  // TODO: consider getting rid of a side effect of storage update to make this a pure function
  if (updateWalletState) {
    const walletStore = this.storage.getWalletStore(this.walletId);
    walletStore.updateLastKnownBlock(blockHeight);
    this.storage.scheduleStateSave();
  }

  return blockHeight;
};
