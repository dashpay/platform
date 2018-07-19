const bmp = require('bitcoin-merkle-proof');

function validateMerkleGeneric(merkleRoot, flags, proofHashes, validationHashes, numTransactions) {
  try {
    return bmp.verify({
      flags,
      hashes: proofHashes,
      include: validationHashes,
      numTransactions,
      merkleRoot,
    });
  } catch (e) {
    return false;
  }
}

function validateMerkleBlock(merkleBlock, txHashes) {
  return validateMerkleGeneric(
    Buffer.from(merkleBlock.header.merkleRoot, 'hex'),
    merkleBlock.flags,
    merkleBlock.hashes.map(h => Buffer.from(h, 'hex')),
    txHashes,
    merkleBlock.numTransactions,
  );
}

const merkleproofs = {
  /**
   * Validate transaction from merkle proofs
   * @param {merkleBlock} merkleBlock merkleblock obtained from dapi for specific tx
   * @param {string} headerHash hash of header from SPV client's validated header chain
   * @param {Array<string>} transactions array of transaction hashes to be validated
   * @returns {bool}
   */
  validateTxProofs: (merkleBlock, headerHash, transactions) => {
    if (headerHash !== merkleBlock.header.hash) {
      return false;
    }

    // todo: temp workaround (refactor bitcoin-merkle-proof post Maithai suggested)
    const merkleClone = JSON.parse(JSON.stringify(merkleBlock));
    merkleClone.header.merkleRoot = Buffer.from(merkleBlock.header.merkleRoot, 'hex').reverse().toString('hex');

    return validateMerkleBlock(merkleClone, transactions);
  },

  validateMnProofs(merkleRoot, merkleFlags, merkleHashes, cbTxHash, totalTransactions) {
    return validateMerkleGeneric(
      merkleRoot,
      merkleFlags,
      merkleHashes,
      [cbTxHash],
      totalTransactions,
    );
  },

};


module.exports = merkleproofs;

