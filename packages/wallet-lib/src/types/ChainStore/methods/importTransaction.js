const { Transaction } = require('@dashevo/dashcore-lib');
const is = require('../../../utils/is');

function importTransaction(transaction, metadata = {}) {
  // Even if transaction is a transaction object, if manglized,
  // it might end up not being a correct instanceof internally.
  if (Array.isArray(transaction)) {
    throw new Error('Will not import an array of transaction');
  }
  const normalizedTransaction = is.string(transaction) ? new Transaction(transaction) : transaction;
  this.state.transactions.set(normalizedTransaction.hash, {
    transaction: normalizedTransaction,
    metadata: {
      blockHash: metadata.blockHash || null,
      height: metadata.height || null,
      isInstantLocked: metadata.isInstantLocked || null,
      isChainLocked: metadata.isChainLocked || null,
    },
  });
  return this.considerTransaction(normalizedTransaction.hash);
}

module.exports = importTransaction;
