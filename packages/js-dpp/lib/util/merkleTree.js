const hash = require('./hash');

/**
 * Builds a merkle tree of all passed hashes
 * @link https://en.bitcoin.it/wiki/Protocol_specification#Merkle_Trees
 * @param {Buffer[]} hashes
 * @returns {Buffer[]} - An array with each level of the tree after the other.
 */
function getMerkleTree(hashes) {
  // Copy all buffers in the tree to avoid unexpected behaviour
  const tree = hashes.map(Buffer.from);

  let j = 0;
  for (let size = hashes.length; size > 1; size = Math.floor((size + 1) / 2)) {
    for (let i = 0; i < size; i += 2) {
      const i2 = Math.min(i + 1, size - 1);
      const buf = Buffer.concat([tree[j + i], tree[j + i2]]);
      tree.push(Buffer.from(hash(buf), 'hex'));
    }
    j += size;
  }

  return tree;
}

/**
 * Copies root of the passed tree to a new Buffer and returns it
 * @param {Buffer[]} merkleTree
 * @returns {Buffer|undefined} - A buffer of the merkle root hash
 */
function getMerkleRoot(merkleTree) {
  if (merkleTree.length === 0) {
    return undefined;
  }
  // Copy root buffer
  return Buffer.from(merkleTree[merkleTree.length - 1]);
}

module.exports = {
  getMerkleTree,
  getMerkleRoot,
};
