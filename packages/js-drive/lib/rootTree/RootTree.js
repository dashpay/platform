const { MerkleTree } = require('merkletreejs');

const hashFunction = require('./hashFunction');

const InvalidLeafIndexError = require('./errors/InvalidLeafIndexError');

class RootTree {
  /**
   *
   * @param {AbstractRootTreeLeaf[]} leaves
   */
  constructor(leaves) {
    leaves.forEach((leaf, index) => {
      if (leaf.getIndex() !== index) {
        throw new InvalidLeafIndexError(leaf, index);
      }
    });

    this.leaves = leaves;

    this.rebuild();
  }

  /**
   * Get root hash
   *
   * @return {Buffer}
   */
  getRootHash() {
    return this.tree.getRoot();
  }

  /**
   *
   * @param {AbstractRootTreeLeaf} leaf
   * @return {Array.<{left:number, right:number, data: Buffer}>}
   */
  getProof(leaf) {
    const hash = this.leafHashes[leaf.getIndex()];

    return this.tree.getProof(hash);
  }

  /**
   * Rebuild root tree with updated leaf hashes
   */
  rebuild() {
    this.leafHashes = this.leaves.map((leaf) => leaf.getHash());
    this.tree = new MerkleTree(this.leafHashes, hashFunction);
  }
}

module.exports = RootTree;
