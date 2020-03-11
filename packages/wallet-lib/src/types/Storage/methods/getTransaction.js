const { TransactionNotInStore } = require('../../../errors');

const getTransaction = function (txid) {
  const search = this.searchTransaction(txid);
  if (!search.found) throw new TransactionNotInStore(txid);
  return search.result;
};

module.exports = getTransaction;
