const { Transaction } = require('@dashevo/dashcore-lib');
/**
 *  Will lookup for predecessors of the inputs's transactions
 * @param {Transaction} transaction
 * @return {Object[]} results - Ordered array of lookup results
 */
module.exports = function lookupInputsPredecessors(transaction) {
  if (!(transaction instanceof Transaction)) throw new Error('Expect input of type Transaction');
  const { inputs } = transaction;
  const lookupResults = [];

  inputs.forEach((input) => {
    const lookupResult = this.lookupByTransactionHash(input.prevTxId.toString('hex'));
    if (lookupResult) lookupResults.push(lookupResult);
  });

  return lookupResults.sort((a, b) => a.pos - b.pos);
};
