/**
 * Import an array of transactions or a transaction object to the store
 * @param transactions
 * @return {boolean}
 * */
const importTransactions = function (transactions) {
  const type = transactions.constructor.name;
  const self = this;

  if (type === 'Object') {
    if (transactions.txid) {
      const transaction = transactions;
      self.importTransaction(transaction);
    } else {
      const transactionsIds = Object.keys(transactions);
      if (transactionsIds.length === 0) {
        throw new Error('Invalid transaction');
      }
      transactionsIds.forEach((id) => {
        const transaction = transactions[id];
        self.importTransaction(transaction);
      });
    }
  } else if (type === 'Array') {
    throw new Error('Not implemented. Please create an issue on github if needed.');
  } else {
    throw new Error('Invalid transaction. Cannot import.');
  }
  return true;
};
module.exports = importTransactions;
