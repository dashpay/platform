const AbstractRootTreeLeaf = require('../rootTree/AbstractRootTreeLeaf');

class PublicKeyToIdentityIdStoreRootTreeLeaf extends AbstractRootTreeLeaf {
  /**
   * @param {MerkDbStore} publicKeyToIdentityIdStore
   */
  constructor(publicKeyToIdentityIdStore) {
    super(PublicKeyToIdentityIdStoreRootTreeLeaf.INDEX);

    this.publicKeyToIdentityIdStore = publicKeyToIdentityIdStore;
  }

  /**
   * Get leaf hash
   *
   * @return {Buffer}
   */
  getHash() {
    return this.publicKeyToIdentityIdStore.getRootHash();
  }
}

PublicKeyToIdentityIdStoreRootTreeLeaf.INDEX = 2;

module.exports = PublicKeyToIdentityIdStoreRootTreeLeaf;
