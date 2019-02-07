const { TransactionNotInStore } = require('../errors');

const getTransaction = function (txid) {
  const { transactions } = this.store;
  if (!transactions[txid]) throw new TransactionNotInStore(txid);
  return this.store.transactions[txid];
};

module.exports = getTransaction;
