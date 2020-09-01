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

  const accountStore = (this.walletType === WALLET_TYPES.SINGLE_ADDRESS)
    ? accountsStore[this.index.toString()]
    : accountsStore[this.BIP44PATH.toString()];

  accountStore.blockHeight = blockHeight;

  return accountStore.blockHeight;
};
