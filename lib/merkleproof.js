const bmp = require('bitcoin-merkle-proof');

function validateMerkleBlock(merkleBlock, transactions) {
  try {
    return bmp.verify({
      flags: merkleBlock.flags,
      hashes: merkleBlock.hashes.map(h => Buffer.from(h, 'hex')),
      include: transactions,
      numTransactions: merkleBlock.numTransactions,
      merkleRoot: Buffer.from(merkleBlock.header.merkleRoot, 'hex'),
    });
  } catch (e) {
    return false;
  }
}

const merkleproofs = {
  /**
   * Validate transaction from merkle proofs
   * @param {merkleBlock} merkleBlock merkleblock obtained from dapi for specific tx
   * @param {string} headerHash hash of header from SPV client's validated header chain
   * @param {Array<string>} transactions array of transaction hashes to be validated
   * @returns {bool}
   */
  validateTransaction: (merkleBlock, headerHash, transactions) => {
    if (headerHash !== merkleBlock.header.hash) {
      return false;
    }

    // todo: temp workaround (refactor bitcoin-merkle-proof post Maithai suggested)
    const merkleClone = JSON.parse(JSON.stringify(merkleBlock));
    merkleClone.header.merkleRoot = Buffer.from(merkleBlock.header.merkleRoot, 'hex').reverse().toString('hex');

    return validateMerkleBlock(merkleClone, transactions);
  },
};


module.exports = merkleproofs;

