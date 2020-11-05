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
}

IdentitiesStoreRootTreeLeaf.INDEX = 1;

module.exports = IdentitiesStoreRootTreeLeaf;
