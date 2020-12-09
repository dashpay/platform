const AbstractRootTreeLeaf = require('../rootTree/AbstractRootTreeLeaf');

class SpentAssetLockTransactionsStoreRootTreeLeaf extends AbstractRootTreeLeaf {
  /**
   * @param {MerkDbStore} spentAssetLockTransactionsStore
   */
  constructor(spentAssetLockTransactionsStore) {
    super(SpentAssetLockTransactionsStoreRootTreeLeaf.INDEX);

    this.store = spentAssetLockTransactionsStore;
  }

  /**
   * Get leaf hash
   *
   * @return {Buffer}
   */
  getHash() {
    return this.store.getRootHash();
  }

  /**
   * Get proof for leaf keys
   *
   * @param {Array<Buffer>} leafKeys
   * @return {Buffer}
   */
  getProof(leafKeys) {
    return this.store.getProof(leafKeys);
  }
}

SpentAssetLockTransactionsStoreRootTreeLeaf.INDEX = 5;

module.exports = SpentAssetLockTransactionsStoreRootTreeLeaf;
