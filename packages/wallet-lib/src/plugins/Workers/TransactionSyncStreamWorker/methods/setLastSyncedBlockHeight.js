const { WALLET_TYPES } = require('../../../../CONSTANTS');

/**
 * Set last synced block height
 *
 * @param  {number} blockHeight
 * @return {number}
 */
module.exports = function setLastSyncedBlockHeight(blockHeight) {
  const { walletId } = this;
  const accountsStore = this.storage.store.wallets[walletId].accounts;

  const accountStore = ([WALLET_TYPES.HDWALLET, WALLET_TYPES.HDPUBLIC].includes(this.walletType))
    ? accountsStore[this.BIP44PATH.toString()]
    : accountsStore[this.index.toString()];

  accountStore.blockHeight = blockHeight;

  return accountStore.blockHeight;
};
