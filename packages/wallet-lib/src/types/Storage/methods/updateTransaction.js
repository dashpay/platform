/**
 * Update a specific transaction information in the store
 * It do not handle any merging right now and write over previous data.
 * @param {Transaction} transaction
 * @return {boolean}
 */
const updateTransaction = function updateTransaction(transaction) {
  if (!transaction) throw new Error('Expected a transaction to update');

  const transactionStore = this.store.transactions;
  const storeTx = transactionStore[transaction.hash];
  if (JSON.stringify(storeTx) !== JSON.stringify(transaction)) {
    transactionStore[transaction.hash] = transaction;
    this.lastModified = Date.now();
  }
  return true;
};
module.exports = updateTransaction;
