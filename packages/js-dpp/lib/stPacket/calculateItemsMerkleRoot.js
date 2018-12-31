const { getMerkleRoot, getMerkleTree } = require('../util/merkleTree');

const calculateItemsHashes = require('./calculateItemsHashes');

/**
 * Calculate merkle root of ST Packet's items
 *
 * @param {{ objects: Buffer[], contracts: Buffer[] }} items
 * @return {string|null}
 */
function calculateItemsMerkleRoot(items) {
  const { contracts, objects } = calculateItemsHashes(items);

  // Always concatenate arrays in bitwise order of their names
  const itemsHashes = contracts.concat(objects);

  if (itemsHashes.length === 0) {
    return null;
  }

  return getMerkleRoot(
    getMerkleTree(itemsHashes),
  ).toString('hex');
}

module.exports = calculateItemsMerkleRoot;
