import WalletStore from '../../WalletStore/WalletStore.js';

const createWalletStore = function createWallet(walletId = 'squawk7700') {
  if (!this.wallets.has(walletId)) {
    this.wallets.set(walletId, new WalletStore(walletId));
    return true;
  }
  return false;
};
export default createWalletStore;