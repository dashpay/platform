/**
 * Search a wallet in store based from it's walletId
 * @param {string} walletId
 * @return {WalletSearchResult}
 */
const searchWallet = function searchWallet(walletId) {
  const search = {
    walletId,
    found: false,
  };
  const store = this.getStore();
  if (store.wallets[walletId]) {
    search.found = true;
    search.result = store.wallets[walletId];
  }
  return search;
};
module.exports = searchWallet;
