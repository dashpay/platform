const { WALLET_TYPES } = require('../../../../CONSTANTS');
/**
 * Return last synced block height
 * @return {number}
 */
module.exports = function getLastSyncedBlockHeight() {
  const { walletId } = this;
  const accountsStore = this.storage.store.wallets[walletId].accounts;

  let { blockHeight } = (this.walletType === WALLET_TYPES.SINGLE_ADDRESS)
    ? accountsStore[this.index.toString()]
    : accountsStore[this.BIP44PATH.toString()];

  // Fix Genesis issue on DCore
  if (blockHeight === 0) blockHeight = 1;

  return blockHeight;
};
