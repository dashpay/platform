/**
 * Set last synced block height
 *
 * @param  {number} blockHeight
 * @param  {boolean} [updateChainStore=false]
 * @return {number}
 */
module.exports = function setLastSyncedBlockHeight(blockHeight, updateChainStore = false) {
  if (this.lastSyncedBlockHeight >= blockHeight) {
    return this.lastSyncedBlockHeight;
  }

  this.lastSyncedBlockHeight = blockHeight;

  // TODO: consider getting rid of a side effect of storage update to make this a pure function
  if (updateChainStore) {
    const chainStore = this.storage.getDefaultChainStore();
    chainStore.updateLastSyncedBlockHeight(blockHeight);
    this.storage.scheduleStateSave();
  }

  return blockHeight;
};
