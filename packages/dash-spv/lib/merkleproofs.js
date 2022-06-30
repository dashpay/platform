const DashUtil = require('@dashevo/dash-util');

const merkleproofs = {
  /**
   * validates an array of tx hashes or Transaction instances
   * against a merkleblock
   * @param {MerkleBlock} merkleBlock - a MerkleBlock instance
   * @param {Transaction[]|string[]} transactions
   * @return {boolean}
   */
  validateTxProofs: (merkleBlock, transactions) => {
    let txToFilter = transactions.slice();
    if (typeof transactions[0] === 'string') {
      txToFilter = txToFilter.map(tx => DashUtil.toHash(tx).toString('hex'));
    }
    return merkleBlock.validMerkleTree
      && txToFilter.filter(tx => merkleBlock.hasTransaction(tx)).length === transactions.length;
  },
};

module.exports = merkleproofs;
