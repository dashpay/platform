const AbstractRootTreeLeaf = require('../rootTree/AbstractRootTreeLeaf');

class IdentitiesStoreRootTreeLeaf extends AbstractRootTreeLeaf {
  /**
   * @param {MerkDbStore} identitiesStore
   */
  constructor(identitiesStore) {
    super(IdentitiesStoreRootTreeLeaf.INDEX);

    this.identitiesStore = identitiesStore;
  }

  /**
   * Get leaf hash
   *
   * @return {Buffer}
   */
  getHash() {
    return this.identitiesStore.getRootHash();
  }

  /**
   * Get proof for leaf keys
   *
   * @param {Array<Buffer>} leafKeys
   * @return {Buffer}
   */
  getProof(leafKeys) {
    return this.identitiesStore.getProof(leafKeys);
  }
}

IdentitiesStoreRootTreeLeaf.INDEX = 1;

module.exports = IdentitiesStoreRootTreeLeaf;
