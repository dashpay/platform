const dashcore = require('bitcore-lib-dash');


const merkleproofs = {

  validateTxProofs: (merkleBlock, transactions) =>
    merkleBlock.validateMerkleBlock() &&
    transactions.filter(t => merkleBlock.hasTransaction(t)).length === transactions.length,

  validateMnProofs(header, flags, hashes, numTransactions, cbTxHash) {
    const merkleBlock = new dashcore.MerkleBlock({
      header,
      numTransactions,
      hashes,
      flags,
    });

    return merkleBlock.validateMerkleBlock() && merkleBlock.hasTransaction(cbTxHash);
  },

};


module.exports = merkleproofs;

