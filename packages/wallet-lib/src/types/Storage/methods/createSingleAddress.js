const { hasProp } = require('../../../utils');

/**
 * Create a new account into a wallet
 * @param {string} walletId
 * @param {string} network
 * @param {string|null} [label]
 * @return {boolean}
 */
module.exports = function createAccount(walletId, network, label = null) {
  if (!hasProp(this.store.wallets, walletId.toString())) {
    if (!this.searchWallet(walletId).found) {
      this.createWallet(walletId, network);
    }
  }

  if (!hasProp(this.store.wallets[walletId].accounts, '0')) {
    this.store.wallets[walletId].accounts['0'] = {
      label,
      network,
      blockHeight: 0, // Used to keep track of local state sync of the account
    };

    return true;
  }
  return false;
};
