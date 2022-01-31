const Dashcore = require('@dashevo/dashcore-lib');
const WalletStore = require('../../WalletStore/WalletStore');

const { testnet } = Dashcore.Networks;

const createWalletStore = function createWallet(walletId = 'squawk7700', network = testnet.toString(), mnemonic = null, type = null) {
  if (!this.wallets.has(walletId)) {
    this.wallets.set(walletId, new WalletStore(walletId));
    return true;
  }
  return false;
};
module.exports = createWalletStore;
