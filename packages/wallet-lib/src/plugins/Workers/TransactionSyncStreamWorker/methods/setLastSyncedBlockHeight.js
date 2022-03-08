/**
 * Set last synced block height
 *
 * @param  {number} blockHeight
 * @return {number}
 */
module.exports = function setLastSyncedBlockHeight(blockHeight) {
  const applicationStore = this.storage.application;
  applicationStore.blockHeight = blockHeight;

  return applicationStore.blockHeight;
};
