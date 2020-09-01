const { TransactionNotInStore } = require('../../../errors');

/**
 * Get a specific transaxtion by it's transaction id
 * @param {string} txid
 * @return {Transaction}
 */
const getTransaction = function getTransaction(txid) {
  const search = this.searchTransaction(txid);
  if (!search.found) throw new TransactionNotInStore(txid);
  return search.result;
};

module.exports = getTransaction;
