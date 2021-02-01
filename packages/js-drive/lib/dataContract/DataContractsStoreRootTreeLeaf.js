const AbstractRootTreeLeaf = require('../rootTree/AbstractRootTreeLeaf');

class DataContractsStoreRootTreeLeaf extends AbstractRootTreeLeaf {
  /**
   * @param {MerkDbStore} dataContractsStore
   */
  constructor(dataContractsStore) {
    super(DataContractsStoreRootTreeLeaf.INDEX);

    this.dataContractsStore = dataContractsStore;
  }

  /**
   * Get leaf hash
   *
   * @return {Buffer}
   */
  getHash() {
    return this.dataContractsStore.getRootHash();
  }

  /**
   * Get proof for leaf keys
   *
   * @param {Array<Buffer>} leafKeys
   * @return {Buffer}
   */
  getProof(leafKeys) {
    return this.dataContractsStore.getProof(leafKeys);
  }
}

DataContractsStoreRootTreeLeaf.INDEX = 3;

module.exports = DataContractsStoreRootTreeLeaf;
