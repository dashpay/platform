const { MerkleTree } = require('merkletreejs');

const hashFunction = require('./hashFunction');
const convertRootTreeProofToBuffer = require('./convertRootTreeProofToBuffer');

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
    const leafHashesAreEmpty = this.leafHashes.find(
      (hash) => !hash.equals(Buffer.alloc(hash.length)),
    ) === undefined;

    return leafHashesAreEmpty ? Buffer.alloc(0) : this.tree.getRoot();
  }

  /**
   *
   * @param {AbstractRootTreeLeaf} leaf
   * @return {Buffer}
   */
  getProof(leaf) {
    const hash = this.leafHashes[leaf.getIndex()];

    return convertRootTreeProofToBuffer(this.tree.getProof(hash));
  }

  /**
   *
   * @param {AbstractRootTreeLeaf} leaf
   * @param {Array<Buffer>} leafKeys
   * @return {Object} proof
   * @return {Buffer} proof.rootTreeProof
   * @return {Buffer} proof.storeTreeProof
   */
  getFullProof(leaf, leafKeys) {
    const storeTreeProof = leaf.getProof(leafKeys);
    const rootTreeProof = this.getProof(leaf);

    return {
      rootTreeProof,
      storeTreeProof,
    };
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
