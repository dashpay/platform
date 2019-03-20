const { getMerkleRoot, getMerkleTree } = require('../util/merkleTree');

const calculateItemsHashes = require('./calculateItemsHashes');

/**
 * Calculate merkle root of ST Packet's items
 *
 * @param {{ documents: Buffer[], contracts: Buffer[] }} items
 * @return {string|null}
 */
function calculateItemsMerkleRoot(items) {
  const { contracts, documents } = calculateItemsHashes(items);

  // Always concatenate arrays in bitwise order of their names
  const itemsHashes = contracts.concat(documents);

  if (itemsHashes.length === 0) {
    return null;
  }

  return getMerkleRoot(
    getMerkleTree(itemsHashes),
  ).toString('hex');
}

module.exports = calculateItemsMerkleRoot;
