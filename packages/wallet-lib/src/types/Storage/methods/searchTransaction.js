/**
 * Search a specific txid in the store
 * @param txid
 * @return {{txid: *, found: boolean}}
 */
const searchTransaction = function (hash) {
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
