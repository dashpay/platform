/**
 * Search a wallet in store based from it's walletId
 * @param walletId
 * @return {{walletId: *, found: boolean}}
 */
const searchWallet = function (walletId) {
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
