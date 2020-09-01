/**
 * Search a specific txid in the store
 * @param {string} hash
 * @return {TransactionSearchResult}
 */
const searchTransaction = function searchTransaction(hash) {
  const search = {
    hash,
    found: false,
  };
  const store = this.getStore();

  if (store.transactions[hash]) {
    const tx = store.transactions[hash];
    if (tx.hash === hash) {
      search.found = true;
      search.result = tx;
    }
  }
  return search;
};
module.exports = searchTransaction;
