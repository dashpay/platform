const { WALLET_TYPES } = require('../../../../CONSTANTS');
/**
 * Return last synced block height
 * @return {number}
 */
module.exports = function getLastSyncedBlockHeight() {
  const { walletId } = this;
  const accountsStore = this.storage.store.wallets[walletId].accounts;

  let { blockHeight } = ([WALLET_TYPES.HDWALLET, WALLET_TYPES.HDPUBLIC].includes(this.walletType))
    ? accountsStore[this.BIP44PATH.toString()]
    : accountsStore[this.index.toString()];

  // Fix Genesis issue on DCore
  if (blockHeight === 0) blockHeight = 1;

  return blockHeight;
};
