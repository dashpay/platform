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

  /**
   * Get proof for leaf keys
   *
   * @param {Array<Buffer>} leafKeys
   * @return {Buffer}
   */
  getProof(leafKeys) {
    return this.publicKeyToIdentityIdStore.getProof(leafKeys);
  }
}

PublicKeyToIdentityIdStoreRootTreeLeaf.INDEX = 2;

module.exports = PublicKeyToIdentityIdStoreRootTreeLeaf;
