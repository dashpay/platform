/**
 * Search a specific txid in the store
 * @param txid
 * @return {{txid: *, found: boolean}}
 */
const searchTransaction = function (txid) {
  const search = {
    txid,
    found: false,
  };
  const store = this.getStore();
  if (store.transactions[txid]) {
    const tx = store.transactions[txid];
    if (tx.txid === txid) {
      search.found = true;
      search.result = tx;
    }
  }
  return search;
};
module.exports = searchTransaction;
