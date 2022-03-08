/**
 * Return last synced block height
 * @return {number}
 */
module.exports = function getLastSyncedBlockHeight() {
  let { blockHeight } = this.storage.application;

  // Fix Genesis issue on DCore
  if (blockHeight === 0) blockHeight = 1;

  return blockHeight;
};
