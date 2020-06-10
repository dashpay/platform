const is = require('../../../../../../utils/is');
/**
 * Will lookup for successors of the txid provided
 * @param {Transaction.hash} hash - hash of the predecessor for the seeked successors
 * @return {Object[]} results - Ordered array of successors results
 */
module.exports = function lookupTxIdSuccessors(hash) {
  if (!is.txid(hash)) throw new Error('Expected lookup parameter to be a txid');
  const lookupResults = [];

  if (!this.transactions.length) return lookupResults;

  this.transactions.forEach((transaction, pos) => {
    const { inputs } = transaction;
    // eslint-disable-next-line no-restricted-syntax
    for (const input of inputs) {
      const prevTxId = input.prevTxId.toString('hex');
      if (prevTxId === hash) {
        lookupResults.push({ tx: transaction, pos });
      }
    }
  });
  return lookupResults;
};
