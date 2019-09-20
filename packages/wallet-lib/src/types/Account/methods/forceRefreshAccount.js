/**
 * Force a refresh of all the addresses informations (utxo, balance, txs...)
 * todo : Use a taskQueue where this would just emit the ask for a refresh.
 * @return {Boolean}
 */
function forceRefreshAccount() {
  const store = this.storage.getStore();
  const addressStore = store.wallets[this.walletId].addresses;
  ['internal', 'external', 'misc'].forEach((type) => {
    Object.keys(addressStore[type]).forEach((path) => {
      addressStore[type][path].fetchedLast = 0;
    });
  });
  return true;
}
module.exports = forceRefreshAccount;
