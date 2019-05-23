const merkleproofs = {

  validateTxProofs: (merkleBlock, transactions) => merkleBlock.validMerkleTree()
    && transactions.filter(t => merkleBlock.hasTransaction(t)).length === transactions.length,
};

module.exports = merkleproofs;
