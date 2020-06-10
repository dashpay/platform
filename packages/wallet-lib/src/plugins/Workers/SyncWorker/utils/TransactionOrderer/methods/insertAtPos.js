/**
 * Add a transaction at a specific position
 * @param {Transaction} transaction
 * @param {number} [pos]
 * @return this
 */
module.exports = function insertAtPos(transaction, pos = undefined) {
  const { transactionIds, transactions } = this;

  if (pos > transactions.length) {
    throw new Error('Cannot insert in an out of range position');
  }

  if (pos === null || pos === undefined) {
    transactions.push(transaction);
    transactionIds.push(transaction.hash);

    return this;
  }

  transactions.splice(pos, 0, transaction);
  transactionIds.splice(pos, 0, transaction.hash);

  return this;
};
