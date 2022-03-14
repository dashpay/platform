function getWalletStore(walletId) {
  if (!this.wallets.has(walletId)) return null;
  return this.wallets.get(walletId);
}
module.exports = getWalletStore;
