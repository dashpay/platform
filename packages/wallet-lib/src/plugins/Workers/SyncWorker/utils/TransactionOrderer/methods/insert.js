const { Transaction } = require('@dashevo/dashcore-lib');
/**
 * @param {Transaction} transaction
 * @return {boolean}
 */
module.exports = function insert(transaction) {
  if (!(transaction instanceof Transaction)) throw new Error('Expect input of type Transaction');
  const { hash } = transaction;
  const { transactionIds, transactions } = this;
  const alreadyInserted = this.lookupByTransactionHash(hash);
  if (!alreadyInserted) {
    if (!transactionIds.length) {
      transactions.push(transaction);
      transactionIds.push(hash);
      return true;
    }

    const predecessorResults = this.lookupInputsPredecessors(transaction);

    const successorResults = this.lookupTxIdSuccessors(hash);
    // If we have a successor, our tx need to be inserted before it
    const successorPos = (successorResults.length) ? successorResults[0].pos : null;

    // If we have predecessor it needs to be inserted after it
    const predecessorPos = (predecessorResults.length)
      ? predecessorResults[predecessorResults.length - 1].pos
      : null;

    let pos = (predecessorPos != null) ? predecessorPos + 1 : null;
    if (successorPos != null) {
      pos = successorPos;
    }

    // If we added a successor before a predecessor we need to extract from
    // successor to predecessor and put it after the predecessor. We then can insert.
    if (successorPos != null && predecessorPos != null && predecessorPos > successorPos) {
      const extractedTx = transactions.splice(successorPos, predecessorPos - successorPos);
      const extractedIds = transactionIds.splice(successorPos, predecessorPos - successorPos);
      transactions.splice(successorPos + 1, 0, ...extractedTx);
      transactionIds.splice(successorPos + 1, 0, ...extractedIds);
      this.insertAtPos(transaction, successorPos + 1);
    } else {
      this.insertAtPos(transaction, pos);
    }
    return true;
  }
  return false;
};
