module.exports = function exportAccounts(walletId) {
  if (!walletId) throw new Error('Expected to export account of a specific walletId');

  if (!this.store.wallets[walletId]) {
    throw new Error(`No wallet with the following walletId found in store : ${walletId}`);
  }
  return this.store.wallets[walletId].accounts;
};
