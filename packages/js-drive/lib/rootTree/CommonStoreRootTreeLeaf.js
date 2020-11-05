const AbstractRootTreeLeaf = require('./AbstractRootTreeLeaf');

class CommonStoreRootTreeLeaf extends AbstractRootTreeLeaf {
  /**
   * @param {MerkDbStore} commonStore
   */
  constructor(commonStore) {
    super(CommonStoreRootTreeLeaf.INDEX);

    this.commonStore = commonStore;
  }

  /**
   * Get leaf hash
   *
   * @return {Buffer}
   */
  getHash() {
    return this.commonStore.getRootHash();
  }
}

CommonStoreRootTreeLeaf.INDEX = 0;

module.exports = CommonStoreRootTreeLeaf;
