/**
 * Search a specific txid's metadata in the store
 * @param {string} hash
 * @return {TransactionMetadataSearchResult}
 */
const searchTransactionMetadata = function searchTransactionMetadata(hash) {
  const search = {
    hash,
    found: false,
  };
  const store = this.getStore();

  if (store.transactionsMetadata[hash]) {
    const txMetadata = store.transactionsMetadata[hash];
    if (txMetadata.hash === hash) {
      search.found = true;
      search.result = txMetadata;
    }
  }
  return search;
};
module.exports = searchTransactionMetadata;
