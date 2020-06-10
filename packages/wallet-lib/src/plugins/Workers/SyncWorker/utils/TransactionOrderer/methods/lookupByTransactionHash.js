const is = require('../../../../../../utils/is');
/**
 * Lookup locally if the transaction hash is already added
 * @param {Transaction.hash} hash
 * @return {{tx: *, pos: number}|null}
 */
module.exports = function lookupByTransactionHash(hash) {
  if (!is.txid(hash)) throw new Error('Expected lookup parameter to be a txid');
  const index = this.transactionIds.indexOf(hash);
  if (index === -1) {
    return null;
  }
  return { tx: this.transactions[index], pos: index };
};
