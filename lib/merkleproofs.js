const dashcore = require('bitcore-lib-dash');


const merkleproofs = {

  validateTxProofs: (merkleBlock, transactions) =>
    merkleBlock.validMerkleTree() &&
    transactions.filter(t => merkleBlock.hasTransaction(t)).length === transactions.length,

  validateMnProofs(header, flags, hashes, numTransactions, cbTxHash) {
    const merkleBlock = new dashcore.MerkleBlock({
      header,
      numTransactions,
      hashes,
      flags,
    });

    return merkleBlock.validMerkleTree() && merkleBlock.hasTransaction(cbTxHash);
  },

};


module.exports = merkleproofs;

