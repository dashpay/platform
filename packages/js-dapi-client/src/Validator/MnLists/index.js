const { MerkleProof } = require('@dashevo/dash-spv');

/**
   * Return true/false depending if mnlistdiff provided proofs is valid
   * @param {BlockHeader} header - bitcore.BlockHeader where the cbTx is included
   * @param {Uint8Array} flags - Merkle flag bits, packed per 8 in a byte
   * @param {Uint32Array} hashes - Merkle hashes in depth-first order
   * @param {Uint32Array} numTransactions - Number of total transactions in blockHash
   * @param {string} cbTxHash - The fully serialized coinbase transaction of blockHash
   * @returns {boolean}
   */
const verifyMnListDiff = (header, flags, hashes, numTransactions,
  cbTxHash) => MerkleProof.validateMnProofs(
  header, flags, hashes, numTransactions, cbTxHash,
);

module.exports = verifyMnListDiff;
